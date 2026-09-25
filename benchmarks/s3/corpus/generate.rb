#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "json"
require "open3"

BASELINE = "bea26dd843f5ecdc8f32cab4dd6faa8740436723"
ROOT = File.expand_path("../../..", __dir__)
OUT = __dir__

SEEDS = [
  {
    "id" => "S3-CORPUS-ENGINE-001",
    "path" => "crates/lightr-engine/src/lib.rs",
    "symbol" => "tests::from_str_roundtrip",
    "package" => "lightr-engine",
    "manifest" => "crates/lightr-engine/Cargo.toml",
    "selector" => "--lib",
    "requirement" => "S3-REQ-ENGINE-001",
    "golden" => "S3-GOLDEN-ENGINE-001",
    "card" => "S3-CARD-ENGINE-001"
  },
  {
    "id" => "S3-CORPUS-STORE-001",
    "path" => "crates/lightr-store/src/lib.rs",
    "symbol" => "tests::default_root_honors_lightr_home",
    "package" => "lightr-store",
    "manifest" => "crates/lightr-store/Cargo.toml",
    "selector" => "--lib",
    "requirement" => "S3-REQ-STORE-001",
    "golden" => "S3-GOLDEN-STORE-001",
    "card" => "S3-CARD-STORE-001"
  }
].freeze

def sha(bytes)
  Digest::SHA256.hexdigest(bytes)
end

def write_json(name, object)
  File.write(File.join(OUT, name), JSON.pretty_generate(object) + "\n")
end

clauses = SEEDS.map do |seed|
  blob, status = Open3.capture2("git", "-C", ROOT, "show", "#{BASELINE}:#{seed.fetch("path")}")
  abort "generate: source unavailable: #{seed.fetch("path")}" unless status.success?

  argv = ["cargo", "test", "--manifest-path", seed.fetch("manifest"), "-p", seed.fetch("package"), *seed.fetch("selector").split(" "), seed.fetch("symbol"), "--", "--exact", "--format", "pretty"]
  stdout, stderr, status = Open3.capture3({ "CARGO_TERM_QUIET" => "true" }, *argv, chdir: ROOT)
  abort "generate: witness failed: #{seed.fetch("id")}" unless status.exitstatus == 0

  seed.slice("id", "path", "symbol", "package", "manifest", "selector", "golden", "card").merge(
    "baseline" => BASELINE,
    "source_sha256" => sha(blob),
    "argv" => argv,
    "expected_exit" => 0,
    "stdout_sha256" => sha(stdout),
    "stderr_sha256" => sha(stderr)
  )
end

write_json("corpus.json", { "schema_version" => 1, "baseline" => BASELINE, "clauses" => clauses })
write_json("requirements.json", {
  "schema_version" => 1,
  "baseline" => BASELINE,
  "requirements" => SEEDS.map { |seed| { "id" => seed.fetch("requirement"), "clause_id" => seed.fetch("id"), "statement" => "Baseline witness #{seed.fetch("symbol")}" } }
})
write_json("goldens.json", {
  "schema_version" => 1,
  "baseline" => BASELINE,
  "goldens" => clauses.map { |clause| { "id" => clause.fetch("golden"), "clause_id" => clause.fetch("id"), "stdout_sha256" => clause.fetch("stdout_sha256"), "stderr_sha256" => clause.fetch("stderr_sha256") } }
})
write_json("cards.json", {
  "schema_version" => 1,
  "baseline" => BASELINE,
  "cards" => SEEDS.map { |seed| { "id" => seed.fetch("card"), "clause_id" => seed.fetch("id"), "title" => seed.fetch("symbol") } }
})
