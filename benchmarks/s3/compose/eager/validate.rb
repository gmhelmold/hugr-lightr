#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"

TOKENS = ["${DOCKER}", "${LIGHTR}", "${FIXTURE_DIR}", "${S3_NAMESPACE}"].freeze

def fail!(reason)
  warn "s3 eager: #{reason}"
  exit 1
end

def argv!(value, field)
  fail!("bad #{field}") unless value.is_a?(Array) && !value.empty? && value.all? { |token| token.is_a?(String) && !token.empty? }
  value.each do |token|
    fail!("unknown placeholder in #{field}") if token.include?("${") && TOKENS.none? { |known| token.include?(known) }
  end
end

path = ARGV.fetch(0) { fail!("scenario path required") }
doc = YAML.safe_load(File.read(path), aliases: false)
fail!("bad document") unless doc.is_a?(Hash) && doc.keys.sort == ["scenarios"]
cases = doc["scenarios"]
fail!("bad scenarios") unless cases.is_a?(Array) && !cases.empty?
ids = {}

cases.each do |entry|
  fields = %w[assertions case_id docker fixture legacy_overlap lightr lightr_engine]
  fail!("bad case") unless entry.is_a?(Hash) && entry.keys.sort == fields
  id = entry["case_id"]
  fail!("bad case id") unless id.is_a?(String) && id.match?(/\A[a-z0-9][a-z0-9._-]{0,127}\z/) && !ids.key?(id)
  ids[id] = true
  fail!("bad fixture") unless entry["fixture"].is_a?(String) && !entry["fixture"].empty?
  %w[docker lightr].each { |field| argv!(entry[field], field) }
  fail!("lightr case lacks --eager") unless entry["lightr"].each_cons(2).any? { |left, right| left == "compose" && right == "up" } && entry["lightr"].include?("--eager")
  fail!("profile not activated") unless [entry["docker"], entry["lightr"]].all? { |argv| argv.each_cons(2).any? { |flag, value| flag == "--profile" && value == "default" } }
  fail!("bad engine") unless entry["lightr_engine"] == "native"
  fail!("bad legacy overlap") unless entry["legacy_overlap"] == "none"
  assertions = entry["assertions"]
  fail!("bad assertions") unless assertions == [{ "id" => "exit-code", "expected" => { "exit_code" => 0 }, "shared" => true }]
end

puts "s3 eager: PASS (#{cases.length} runner-compatible cases)"
