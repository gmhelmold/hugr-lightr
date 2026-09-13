#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "json"
require "open3"

BASELINE = "bea26dd843f5ecdc8f32cab4dd6faa8740436723"
HEX = /\A[0-9a-f]{64}\z/

def fail!(reason)
  warn "s3 corpus: #{reason}"
  exit 1
end

def sha(bytes)
  Digest::SHA256.hexdigest(bytes)
end

def load_table(dir, name, key)
  value = JSON.parse(File.read(File.join(dir, name)))
  fail!("bad #{name}") unless value.is_a?(Hash) && value.keys.sort == ["baseline", key, "schema_version"].sort && value["schema_version"] == 1 && value["baseline"] == BASELINE && value[key].is_a?(Array) && !value[key].empty?
  value[key]
rescue JSON::ParserError, SystemCallError
  fail!("bad #{name}")
end

def nonempty_string?(value)
  value.is_a?(String) && !value.empty?
end

def sha256?(value)
  nonempty_string?(value) && HEX.match?(value)
end

def validate_rows(rows, name, fields)
  rows.each do |row|
    fail!("bad #{name} row") unless row.is_a?(Hash) && row.keys.sort == fields.sort
  end
end

def validate_clauses(clauses)
  fields = %w[id baseline path symbol source_sha256 package manifest selector argv expected_exit stdout_sha256 stderr_sha256 golden card]
  validate_rows(clauses, "clause", fields)
  clauses.each do |clause|
    fail!("bad clause") unless clause["baseline"] == BASELINE
    fail!("bad clause") unless clause.values_at("id", "path", "symbol", "package", "manifest", "golden", "card").all? { |value| nonempty_string?(value) }
    fail!("bad selector") unless clause["selector"] == "--lib" || (nonempty_string?(clause["selector"]) && (clause["selector"].match?(/\A--bin [^ ]+\z/) || clause["selector"].match?(/\A--test [^ ]+\z/)))
    fail!("bad hashes") unless clause.values_at("source_sha256", "stdout_sha256", "stderr_sha256").all? { |value| sha256?(value) }
    fail!("bad argv") unless clause["argv"].is_a?(Array) && clause["argv"].all? { |value| nonempty_string?(value) } && clause["argv"] == expected_argv(clause) && clause["expected_exit"] == 0
  end
end

def validate_requirements(rows)
  validate_rows(rows, "requirement", %w[id clause_id statement])
  rows.each { |row| fail!("bad requirement row") unless row.values_at("id", "clause_id", "statement").all? { |value| nonempty_string?(value) } }
end

def validate_goldens(rows)
  validate_rows(rows, "golden", %w[id clause_id stdout_sha256 stderr_sha256])
  rows.each { |row| fail!("bad golden row") unless nonempty_string?(row["id"]) && nonempty_string?(row["clause_id"]) && sha256?(row["stdout_sha256"]) && sha256?(row["stderr_sha256"]) }
end

def validate_cards(rows)
  validate_rows(rows, "card", %w[id clause_id title])
  rows.each { |row| fail!("bad card row") unless row.values_at("id", "clause_id", "title").all? { |value| nonempty_string?(value) } }
end

def index(rows, table)
  ids = rows.map { |row| row["id"] }
  fail!("duplicate #{table} mapping") unless ids.all? { |id| id.is_a?(String) && !id.empty? } && ids.uniq.length == ids.length
  rows.to_h { |row| [row.fetch("id"), row] }
end

def expected_argv(clause)
  ["cargo", "test", "--manifest-path", clause.fetch("manifest"), "-p", clause.fetch("package"), *clause.fetch("selector").split(" "), clause.fetch("symbol"), "--", "--exact", "--format", "pretty"]
end

begin
  dir = ARGV.shift || fail!("corpus directory required")
  baseline_worktree = ARGV.shift || fail!("baseline worktree required")
  cargo = ENV.fetch("S3_CARGO", "cargo")
  fail!("baseline worktree unavailable") unless File.directory?(baseline_worktree)

  clauses = load_table(dir, "corpus.json", "clauses")
  requirements = load_table(dir, "requirements.json", "requirements")
  goldens = load_table(dir, "goldens.json", "goldens")
  cards = load_table(dir, "cards.json", "cards")
  validate_clauses(clauses)
  validate_requirements(requirements)
  validate_goldens(goldens)
  validate_cards(cards)
  requirement_index = index(requirements, "requirement")
  golden_index = index(goldens, "golden")
  card_index = index(cards, "card")
  index(clauses, "clause")

  fail!("duplicate requirement mapping") unless requirements.map { |row| row["clause_id"] }.uniq.length == clauses.length && requirements.length == clauses.length
  fail!("duplicate golden mapping") unless goldens.map { |row| row["clause_id"] }.uniq.length == clauses.length && goldens.length == clauses.length
  fail!("duplicate card mapping") unless cards.map { |row| row["clause_id"] }.uniq.length == clauses.length && cards.length == clauses.length

  clauses.each do |clause|
    fail!("missing requirement mapping") unless requirement_index.values.any? { |row| row["clause_id"] == clause["id"] }
    golden = golden_index[clause["golden"]]
    fail!("missing golden mapping") unless golden && golden["clause_id"] == clause["id"] && golden["stdout_sha256"] == clause["stdout_sha256"] && golden["stderr_sha256"] == clause["stderr_sha256"]
    card = card_index[clause["card"]]
    fail!("missing card mapping") unless card && card["clause_id"] == clause["id"]

    source, source_status = Open3.capture2("git", "-C", baseline_worktree, "show", "#{BASELINE}:#{clause["path"]}")
    fail!("post-baseline path: #{clause["path"]}") unless source_status.success?
    fail!("byte drift: #{clause["path"]}") unless sha(source) == clause["source_sha256"]

    argv = clause["argv"].dup
    argv[0] = cargo
    stdout, stderr, status = Open3.capture3({ "CARGO_TERM_QUIET" => "true" }, *argv, chdir: baseline_worktree)
    fail!("bad result: #{clause["id"]}") unless status.exitstatus == clause["expected_exit"]
    combined = stdout + stderr
    fail!("unexecuted witness: #{clause["id"]}") unless combined.include?("running 1 test") && combined.match?(/test #{Regexp.escape(clause["symbol"])} \.\.\. ok/)
    fail!("stdout byte drift: #{clause["id"]}") unless sha(stdout) == clause["stdout_sha256"]
    fail!("stderr byte drift: #{clause["id"]}") unless sha(stderr) == clause["stderr_sha256"]
  end

  puts "s3 corpus: PASS (#{clauses.length} witnesses)"
rescue StandardError => error
  fail!("internal error: #{error.class}")
end
