#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "fileutils"
require "json"
require "open3"
require "tmpdir"

root = File.expand_path("../../..", __dir__)
verify = File.join(__dir__, "verify.rb")
Dir.mktmpdir("s3-corpus-teeth") do |tmp|
  worktree = root
  corpus = File.join(tmp, "corpus")
  FileUtils.mkdir_p(corpus)
  baseline = "bea26dd843f5ecdc8f32cab4dd6faa8740436723"
  path = "crates/lightr-engine/src/lib.rs"
  source, source_status = Open3.capture2("git", "-C", worktree, "show", "#{baseline}:#{path}")
  abort "fixture baseline source unavailable" unless source_status.success?
  digest = Digest::SHA256.hexdigest(source)
  stdout = "running 1 test\ntest witness ... ok\n"
  hashes = { "stdout_sha256" => Digest::SHA256.hexdigest(stdout), "stderr_sha256" => Digest::SHA256.hexdigest("") }
  clause = { "id" => "C", "baseline" => baseline, "path" => path, "symbol" => "witness", "source_sha256" => digest, "package" => "crate", "manifest" => "crate/Cargo.toml", "selector" => "--lib", "argv" => ["cargo", "test", "--manifest-path", "crate/Cargo.toml", "-p", "crate", "--lib", "witness", "--", "--exact", "--format", "pretty"], "expected_exit" => 0, "golden" => "G", "card" => "K" }.merge(hashes)
  golden = { "id" => "G", "clause_id" => "C" }.merge(hashes)
  tables = { "corpus.json" => { "schema_version" => 1, "baseline" => baseline, "clauses" => [clause] }, "requirements.json" => { "schema_version" => 1, "baseline" => baseline, "requirements" => [{ "id" => "R", "clause_id" => "C", "statement" => "witness" }] }, "goldens.json" => { "schema_version" => 1, "baseline" => baseline, "goldens" => [golden] }, "cards.json" => { "schema_version" => 1, "baseline" => baseline, "cards" => [{ "id" => "K", "clause_id" => "C", "title" => "witness" }] } }
  write_tables = lambda { |value| value.each { |name, body| File.write(File.join(corpus, name), JSON.generate(body)) } }
  copy_tables = lambda { JSON.parse(JSON.generate(tables)) }
  write_tables.call(tables)
  cargo = File.join(tmp, "cargo")
  File.write(cargo, "#!/bin/sh\nprintf 'running 1 test\\ntest witness ... ok\\n'\n")
  FileUtils.chmod("+x", cargo)
  output, status = Open3.capture2e({ "S3_CARGO" => cargo }, "ruby", verify, corpus, worktree)
  abort "positive control failed: #{output}" unless status.success?

  assert_rejected = lambda do |name, expected, mutated|
    write_tables.call(mutated)
    output, status = Open3.capture2e({ "S3_CARGO" => cargo }, "ruby", verify, corpus, worktree)
    abort "#{name} tooth passed: #{output}" if status.success?
    abort "#{name} tooth unnamed: #{output}" unless output.lines.all? { |line| line.start_with?("s3 corpus:") } && output.include?(expected)
  end

  malformed = copy_tables.call
  malformed["requirements.json"]["requirements"] = ["scalar"]
  assert_rejected.call("scalar row", "s3 corpus: bad requirement row", malformed)
  malformed = copy_tables.call
  malformed["requirements.json"]["requirements"][0].delete("id")
  assert_rejected.call("missing id", "s3 corpus: bad requirement row", malformed)
  malformed = copy_tables.call
  malformed["goldens.json"]["goldens"][0].delete("clause_id")
  assert_rejected.call("missing clause field", "s3 corpus: bad golden row", malformed)
  malformed = copy_tables.call
  malformed["cards.json"] = []
  assert_rejected.call("malformed mapping table", "s3 corpus: bad cards.json", malformed)

  malformed = copy_tables.call
  malformed["corpus.json"]["clauses"][0]["stdout_sha256"] = "0" * 64
  malformed["goldens.json"]["goldens"][0]["stdout_sha256"] = "0" * 64
  write_tables.call(malformed)
  output, status = Open3.capture2e({ "S3_CARGO" => cargo }, "ruby", verify, corpus, worktree)
  abort "hash tooth failed: #{output}" if status.success? || !output.include?("s3 corpus: stdout byte drift: C")
end
puts "s3 corpus teeth: PASS"
