#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"

TOKENS = ["${DOCKER}", "${LIGHTR}", "${FIXTURE_DIR}", "${S3_NAMESPACE}"].freeze
ASSERTIONS = %w[
  command-exit service-port-readiness dependency-ordering selected-profiles
  project-isolation teardown logs health-state
].freeze

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
fail!("bad document") unless doc.is_a?(Hash) && doc.keys.sort == %w[cases schema_version] && doc["schema_version"] == 1
cases = doc["cases"]
fail!("bad cases") unless cases.is_a?(Array) && !cases.empty?
ids = {}

cases.each do |entry|
  fields = %w[assertions case_id docker eager fixture healthcheck lightr readiness resources teardown]
  fail!("bad case") unless entry.is_a?(Hash) && entry.keys.sort == fields
  id = entry["case_id"]
  fail!("bad case id") unless id.is_a?(String) && id.match?(/\A[a-z0-9][a-z0-9._-]{0,127}\z/) && !ids.key?(id)
  ids[id] = true
  fail!("case not eager") unless entry["eager"] == true
  fail!("bad fixture") unless entry["fixture"].is_a?(String) && !entry["fixture"].empty?
  %w[docker lightr readiness].each { |field| argv!(entry[field], field) }
  fail!("lightr case lacks --eager") unless entry["lightr"].each_cons(2).any? { |left, right| left == "compose" && right == "up" } && entry["lightr"].include?("--eager")
  fail!("bad teardown") unless entry["teardown"].is_a?(Hash) && entry["teardown"].keys.sort == %w[docker lightr]
  entry["teardown"].each { |tool, argv| argv!(argv, "teardown.#{tool}") }
  fail!("missing health assertion") unless entry["healthcheck"] == true
  resources = entry["resources"]
  fail!("bad resources") unless resources.is_a?(Array) && !resources.empty? && resources.all? { |name| name.is_a?(String) && name.match?(/\A\$\{S3_NAMESPACE\}(?:-[A-Za-z0-9._-]+)?\z/) }
  assertions = entry["assertions"]
  fail!("bad assertions") unless assertions.is_a?(Array) && assertions.length == ASSERTIONS.length
  got = assertions.map { |assertion| assertion.is_a?(Hash) && assertion.keys.sort == %w[id shared] && assertion["shared"] == true ? assertion["id"] : nil }
  fail!("eager assertions incomplete") unless got.sort == ASSERTIONS.sort
end

puts "s3 eager: PASS (#{cases.length} cases)"
