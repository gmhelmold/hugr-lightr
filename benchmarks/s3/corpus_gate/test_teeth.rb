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
  tables.each { |name, body| File.write(File.join(corpus, name), JSON.generate(body)) }
  cargo = File.join(tmp, "cargo")
  File.write(cargo, "#!/bin/sh\nprintf 'running 1 test\\ntest witness ... ok\\n'\n")
  FileUtils.chmod("+x", cargo)
  output, status = Open3.capture2e({ "S3_CARGO" => cargo }, "ruby", verify, corpus, worktree)
  abort "positive control failed: #{output}" unless status.success?
  clause["stdout_sha256"] = "0" * 64
  tables["goldens.json"]["goldens"][0]["stdout_sha256"] = "0" * 64
  File.write(File.join(corpus, "corpus.json"), JSON.generate(tables["corpus.json"]))
  File.write(File.join(corpus, "goldens.json"), JSON.generate(tables["goldens.json"]))
  output, status = Open3.capture2e({ "S3_CARGO" => cargo }, "ruby", verify, corpus, worktree)
  abort "hash tooth failed: #{output}" if status.success? || !output.include?("s3 corpus: stdout byte drift: C")
end
puts "s3 corpus teeth: PASS"
