#!/usr/bin/env ruby
# Frozen S3-5A fixture parser. It validates only contract structure and edges.
require "json"
require "tmpdir"
require "fileutils"
require "rbconfig"

ROOT = File.expand_path(__dir__)

def fail_contract(message)
  abort("volume contract: #{message}")
end

def load_json(path)
  JSON.parse(File.read(path))
rescue Errno::ENOENT
  fail_contract("missing #{path}")
rescue JSON::ParserError => error
  fail_contract("malformed JSON #{path}: #{error.message}")
end

def required!(object, fields, label)
  fail_contract("#{label} must be object") unless object.is_a?(Hash)
  missing = fields.reject { |field| object.key?(field) }
  fail_contract("#{label} missing #{missing.join(",")}") unless missing.empty?
end

def event_index_after(events, event, start_index, run_id = nil)
  events.each_with_index do |candidate, index|
    next if index <= start_index || candidate["event"] != event
    return index if run_id.nil? || candidate["run_id"] == run_id
  end
  nil
end

def durable_after?(events, start_index)
  write = event_index_after(events, "write.tmp", start_index)
  sync = write && event_index_after(events, "sync_all.tmp", write)
  rename = sync && event_index_after(events, "rename.owners", sync)
  fsync = rename && event_index_after(events, "fsync.parent", rename)
  fsync
end

def with_corpus
  Dir.mktmpdir("volume-contract-") do |directory|
    schema = File.join(directory, "schema.json")
    fixtures = File.join(directory, "fixtures.json")
    goldens = File.join(directory, "goldens.json")
    [schema, fixtures, goldens].zip(%w[schema.json fixtures.json goldens.json]).each do |destination, source|
      FileUtils.cp(File.join(ROOT, source), destination)
    end
    yield schema, fixtures, goldens
  end
end

def registry_matches_owner?(registry, owner)
  process = owner.fetch("process")
  registry["run_id"] == owner["run_id"] &&
    registry["nonce"] == owner["nonce"] &&
    registry["pid"] == process["pid"] &&
    registry["process_start_token"] == process["start_token"] &&
    registry["mount_id"] == owner["mount_id"]
end

def registry_matches_active_owner?(registry, owner)
  registry["run_id"] == owner["run_id"] &&
    registry["nonce"] == owner["nonce"] &&
    registry["pid"] == owner["pid"] &&
    registry["process_start_token"] == owner["process_start_token"] &&
    registry["mount_id"] == owner["mount_id"]
end

def expected_non_fault_outcome(events, id)
  return "released" if events.any? { |event| event["event"] == "owner.remove" }
  return "refused:spawn" if events.any? { |event| event["event"] == "pending.remove" }
  return "recovered" if events.any? { |event| event["event"] == "recover.active" }
  if events.any? { |event| %w[rm prune].include?(event["event"]) }
    return "refused:ambiguous-registry" if events.any? { |event| event["event"] == "registry.observe" && event["result"] == "unreadable" }
    return "refused:non-empty-owners"
  end
  fail_contract("#{id}: non-fault trace has no contract outcome")
end

def owner_identity(owner)
  [owner["run_id"], owner["nonce"], owner["process_start_token"]]
end

def reduce_final_owners(initial_owners, events)
  owners = initial_owners.map { |owner| owner.is_a?(Hash) ? owner.dup : owner }
  events.each do |event|
    case event["event"]
    when "owner.active"
      process = event.fetch("process")
      owner = {
        "phase" => "active",
        "nonce" => event.fetch("nonce"),
        "run_id" => event.fetch("run_id"),
        "pid" => process.fetch("pid"),
        "process_start_token" => process.fetch("start_token"),
        "mount_id" => event.fetch("mount_id")
      }
      # Promotion replaces only its pending nonce. Existing active owners are
      # distinct until a full-identity owner.remove or recover.active removes them.
      owners.reject! { |candidate| candidate.is_a?(Hash) && candidate["phase"] == "pending" && candidate["nonce"] == owner["nonce"] }
      owners << owner unless owners.any? { |candidate| candidate.is_a?(Hash) && candidate["phase"] == "active" && owner_identity(candidate) == owner_identity(owner) }
    when "owner.remove"
      target = [event.fetch("run_id"), event.fetch("nonce"), event.fetch("process_start_token")]
      index = owners.index { |candidate| candidate.is_a?(Hash) && candidate["phase"] == "active" && owner_identity(candidate) == target }
      owners.delete_at(index) if index
    when "recover.active"
      target = [event.fetch("run_id"), event.fetch("nonce"), event.fetch("process_start_token")]
      owners.reject! { |candidate| candidate.is_a?(Hash) && candidate["phase"] == "active" && owner_identity(candidate) == target }
    end
  end
  owners
end

schema_path, fixtures_path, goldens_path = ARGV
if schema_path == "--self-test" && fixtures_path.nil? && goldens_path.nil?
  with_corpus do |schema, fixtures, goldens|
    fail_contract("self-test clean corpus failed") unless system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(goldens)
    mutated.fetch("goldens").find { |golden| golden.fetch("id") == "lock-loss-refuses-without-deletion" }["outcome"] = "released"
    File.write(goldens, JSON.generate(mutated))
    fail_contract("self-test fault mismatch passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    File.write(fixtures, "{")
    fail_contract("self-test malformed fixture passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(fixtures)
    trace = mutated.fetch("fixtures").find { |fixture| fixture.fetch("id") == "normal-teardown" }.fetch("trace")
    trace.reject! { |event| event["event"] == "registry.terminal" }
    File.write(fixtures, JSON.generate(mutated))
    fail_contract("self-test missing terminal passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(fixtures)
    trace = mutated.fetch("fixtures").find { |fixture| fixture.fetch("id") == "concurrent-rm-prune-refuse-active" }.fetch("trace")
    trace.reject! { |event| event["event"] == "lock.acquire" && event["actor"] == "rm" }
    File.write(fixtures, JSON.generate(mutated))
    fail_contract("self-test missing required lock passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(fixtures)
    trace = mutated.fetch("fixtures").find { |fixture| fixture.fetch("id") == "normal-teardown" }.fetch("trace")
    trace.reject! { |event| event["event"] == "lock.release" }
    File.write(fixtures, JSON.generate(mutated))
    fail_contract("self-test missing lock release passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(fixtures)
    registry = mutated.fetch("fixtures").find { |fixture| fixture.fetch("id") == "normal-teardown" }.fetch("trace").find { |event| event["event"] == "registry.active" }
    registry["process_start_token"] = "linux:200:99"
    File.write(fixtures, JSON.generate(mutated))
    fail_contract("self-test registry process token mismatch passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(goldens)
    golden = mutated.fetch("goldens").find { |candidate| candidate.fetch("id") == "concurrent-rm-prune-refuse-active" }
    golden.fetch("observations")["lock_serialized"] = false
    File.write(goldens, JSON.generate(mutated))
    fail_contract("self-test concurrent lock serialization mismatch passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(goldens)
    golden = mutated.fetch("goldens").find { |candidate| candidate.fetch("id") == "normal-teardown" }
    golden["outcome"] = "refused:non-empty-owners"
    File.write(goldens, JSON.generate(mutated))
    fail_contract("self-test normal outcome mismatch passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(goldens)
    golden = mutated.fetch("goldens").find { |candidate| candidate.fetch("id") == "normal-teardown" }
    golden.fetch("state")["owners"] = ["active:nonce_a"]
    File.write(goldens, JSON.generate(mutated))
    fail_contract("self-test normal final owners mismatch passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(fixtures)
    loss = mutated.fetch("fixtures").find { |fixture| fixture.fetch("id") == "lock-loss-refuses-without-deletion" }.fetch("trace").find { |event| event["event"] == "lock.loss" }
    loss["actor"] = "prune"
    File.write(fixtures, JSON.generate(mutated))
    fail_contract("self-test lock loss interval mismatch passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(fixtures)
    registry = mutated.fetch("fixtures").find { |fixture| fixture.fetch("id") == "terminal-registry-positive-death-proof" }.fetch("initial").fetch("registries").first
    registry["process_start_token"] = "linux:200:99"
    File.write(fixtures, JSON.generate(mutated))
    fail_contract("self-test recovery registry identity mismatch passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(fixtures)
    terminal = mutated.fetch("fixtures").find { |fixture| fixture.fetch("id") == "terminal-registry-positive-death-proof" }.fetch("trace").find { |event| event["event"] == "registry.observe" }
    terminal["run_id"] = "run-b"
    File.write(fixtures, JSON.generate(mutated))
    fail_contract("self-test recovery terminal observation identity mismatch passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(fixtures)
    trace = mutated.fetch("fixtures").find { |fixture| fixture.fetch("id") == "shared-mounts" }.fetch("trace")
    second_remove = trace.rindex { |event| event["event"] == "owner.remove" }
    trace[second_remove] = trace.find { |event| event["event"] == "owner.remove" }.dup
    File.write(fixtures, JSON.generate(mutated))
    fail_contract("self-test shared duplicate owner removal passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end

  with_corpus do |schema, fixtures, goldens|
    mutated = load_json(fixtures)
    owner = mutated.fetch("fixtures").find { |fixture| fixture.fetch("id") == "shared-mounts" }.fetch("initial").fetch("owners").last
    owner["nonce"] = "nonce_a"
    File.write(fixtures, JSON.generate(mutated))
    fail_contract("self-test nonce collision owner removal passed") if system(RbConfig.ruby, __FILE__, schema, fixtures, goldens)
  end
  puts "volume contract self-test: OK"
  exit 0
end
schema_path ||= File.join(ROOT, "schema.json")
fixtures_path ||= File.join(ROOT, "fixtures.json")
goldens_path ||= File.join(ROOT, "goldens.json")
schema = load_json(schema_path)
fixtures_document = load_json(fixtures_path)
goldens_document = load_json(goldens_path)

fail_contract("unsupported schema version") unless schema["schema_version"] == 1
fixtures = fixtures_document["fixtures"]
goldens = goldens_document["goldens"]
fail_contract("fixtures must be non-empty array") unless fixtures.is_a?(Array) && !fixtures.empty?
fail_contract("goldens must be non-empty array") unless goldens.is_a?(Array) && !goldens.empty?
fixture_ids = fixtures.map { |fixture| fixture["id"] }
golden_ids = goldens.map { |golden| golden["id"] }
fail_contract("duplicate fixture ID") unless fixture_ids.uniq.length == fixture_ids.length
fail_contract("fixture/golden ID mismatch") unless fixture_ids.sort == golden_ids.sort
goldens_by_id = goldens.to_h { |golden| [golden["id"], golden] }

fixtures.each do |fixture|
  required!(fixture, schema.fetch("fixture_required"), "fixture")
  id = fixture.fetch("id")
  events = fixture.fetch("trace")
  fail_contract("#{id}: trace must be non-empty array") unless events.is_a?(Array) && !events.empty?
  required!(fixture.fetch("initial"), ["volume_exists", "owners"], "#{id}: initial")
  golden = goldens_by_id.fetch(id)
  unless events.any? { |event| event["event"] == "fault" }
    required!(golden, schema.fetch("golden_required"), "#{id}: golden")
    state = golden.fetch("state")
    observations = golden.fetch("observations")
    required!(state, schema.fetch("golden_state_required"), "#{id}: golden state")
    fail_contract("#{id}: golden outcome must be non-empty string") unless golden.fetch("outcome").is_a?(String) && !golden.fetch("outcome").empty?
    fail_contract("#{id}: golden state volume_exists must be boolean") unless [true, false].include?(state.fetch("volume_exists"))
    fail_contract("#{id}: golden state owners must be array") unless state.fetch("owners").is_a?(Array)
    fail_contract("#{id}: golden observations must be non-empty object") unless observations.is_a?(Hash) && !observations.empty?
    expected_outcome = expected_non_fault_outcome(events, id)
    fail_contract("#{id}: golden outcome expected #{expected_outcome}") unless golden.fetch("outcome") == expected_outcome
    expected_owners = reduce_final_owners(fixture.fetch("initial").fetch("owners"), events)
    fail_contract("#{id}: golden volume existence mismatch") unless state.fetch("volume_exists") == fixture.fetch("initial").fetch("volume_exists")
    fail_contract("#{id}: golden final owners mismatch") unless state.fetch("owners") == expected_owners
    if id == "concurrent-rm-prune-refuse-active"
      fail_contract("#{id}: golden must require lock_serialized=true") unless observations["lock_serialized"] == true
    end
  end

  events.each_with_index do |event, index|
    required!(event, ["event"], "#{id}: event #{index}")
    fields = schema.fetch("events")[event.fetch("event")]
    fail_contract("#{id}: unknown event #{event["event"]}") unless fields
    required!(event, fields, "#{id}: #{event["event"]}")
    next unless event["event"] == "fault"

    fail_contract("#{id}: invalid fault point") unless schema.fetch("fault_points").include?(event.fetch("point"))
    prior = events[index - 1]
    fail_contract("#{id}: fault must immediately follow point") unless prior && prior["event"] == event.fetch("point")
    golden = goldens_by_id.fetch(id)
    fail_contract("#{id}: fault golden mismatch") unless golden["outcome"] == "refused:#{event["point"]}"
    state = golden["state"]
    observations = golden["observations"]
    fail_contract("#{id}: fault deletes volume") unless state && state["volume_exists"] == true && observations && observations["delete_attempted"] == false
  end

  lock_holder = nil
  events.each do |event|
    case event["event"]
    when "lock.acquire"
      fail_contract("#{id}: overlapping lock acquire") if lock_holder
      lock_holder = event.fetch("actor")
    when "lock.release"
      fail_contract("#{id}: lock release without matching acquire") unless lock_holder == event.fetch("actor")
      lock_holder = nil
    when "lock.loss"
      fail_contract("#{id}: lock loss outside matching lock interval") unless lock_holder == event.fetch("actor")
      lock_holder = nil
    when "rm", "prune"
      fail_contract("#{id}: #{event["event"]} outside matching lock interval") unless lock_holder == event.fetch("actor")
    end
  end
  # Fault traces terminate at the injected operation; complete traces must
  # release every acquired lock instead of relying on process exit.
  fail_contract("#{id}: lock acquire missing matching release for #{lock_holder}") if lock_holder && !events.any? { |event| event["event"] == "fault" }

  events.each_with_index do |event, index|
    next unless event["event"] == "owner.remove"
    run_id = event.fetch("run_id")
    nonce = event.fetch("nonce")
    owner = events.each_index.find { |candidate| candidate < index && events[candidate]["event"] == "owner.active" && events[candidate]["run_id"] == run_id && events[candidate]["nonce"] == nonce }
    fail_contract("#{id}: owner removal lacks matching active owner for #{run_id}") unless owner
    process = events[owner].fetch("process")
    required!(process, ["pid", "start_token"], "#{id}: active owner process")
    fail_contract("#{id}: owner removal process token mismatch for #{run_id}") unless event.fetch("process_start_token") == process.fetch("start_token")
    active = events.each_index.find { |candidate| candidate > owner && candidate < index && events[candidate]["event"] == "registry.active" && registry_matches_owner?(events[candidate], events[owner]) }
    fail_contract("#{id}: active registry identity mismatch for #{run_id}") unless active
    terminal = events.each_index.find { |candidate| candidate > active && candidate < index && events[candidate]["event"] == "registry.terminal" && events[candidate]["run_id"] == run_id && events[candidate]["nonce"] == nonce }
    terminal_sync = terminal && event_index_after(events, "registry.sync", terminal, run_id)
    fail_contract("#{id}: owner removal lacks active-terminal-sync registry sequence for #{run_id}") unless terminal_sync && terminal_sync < index
  end

  events.each_with_index do |event, index|
    next unless event["event"] == "barrier.release"
    run_id = event.fetch("run_id")
    active = events.each_index.find { |candidate| candidate < index && events[candidate]["event"] == "owner.active" && events[candidate]["run_id"] == run_id }
    fail_contract("#{id}: barrier release lacks active owner") unless active
    active_process = events[active].fetch("process")
    required!(active_process, ["pid", "start_token"], "#{id}: active process")
    durable = durable_after?(events, active)
    registry = durable && event_index_after(events, "registry.active", durable, run_id)
    registry_sync = registry && event_index_after(events, "registry.sync", registry, run_id)
    fail_contract("#{id}: barrier release precedes durable active registry") unless registry_sync && registry_sync < index
  end

  recovery = events.find { |event| event["event"] == "recover.active" }
  next unless recovery
  nonce = recovery.fetch("nonce")
  run_id = recovery.fetch("run_id")
  process_start_token = recovery.fetch("process_start_token")
  owners = fixture.fetch("initial").fetch("owners")
  registries = fixture.fetch("initial")["registries"]
  owner = owners.find { |candidate| candidate.is_a?(Hash) && candidate["phase"] == "active" && owner_identity(candidate) == [run_id, nonce, process_start_token] }
  fail_contract("#{id}: recovery lacks full active owner") unless owner
  required!(owner, schema.fetch("active_owner_required"), "#{id}: active owner")
  registry = registries&.find { |candidate| candidate.is_a?(Hash) && candidate["nonce"] == nonce && candidate["run_id"] == owner["run_id"] }
  fail_contract("#{id}: recovery lacks matching full registry") unless registry
  required!(registry, schema.fetch("registry_required"), "#{id}: registry")
  fail_contract("#{id}: recovery registry identity mismatch") unless registry_matches_active_owner?(registry, owner)
  terminal_observations = events.select { |event| event["event"] == "registry.observe" && event["result"] == "terminal" }
  terminal = terminal_observations.any? { |event| event["run_id"] == owner["run_id"] && event["nonce"] == nonce }
  fail_contract("#{id}: recovery terminal observation identity mismatch") if !terminal_observations.empty? && !terminal
  token_proof = events.any? { |event| event["event"] == "registry.observe" && event["nonce"] == nonce && event["result"] == "readable" } && events.any? { |event| event["event"] == "process.observe" && event["pid"] == owner["pid"] && %w[absent mismatch].include?(event["result"]) }
  fail_contract("#{id}: recovery lacks positive death proof") unless terminal || token_proof
end

puts "volume contract: OK (#{fixtures.length} fixtures)"
