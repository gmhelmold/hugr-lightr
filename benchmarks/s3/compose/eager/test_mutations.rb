#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "open3"
require "tmpdir"
require "yaml"

root = File.expand_path("../../../..", __dir__)
source = File.join(__dir__, "scenarios.yaml")
validator = File.join(__dir__, "validate.rb")

def reject!(validator, path, name, expected)
  output, status = Open3.capture2e("ruby", validator, path)
  abort "#{name} mutation passed: #{output}" if status.success?
  abort "#{name} mutation wrong failure: #{output}" unless output.include?(expected)
end

Dir.mktmpdir("s3-eager-teeth") do |dir|
  clean = File.join(dir, "scenarios.yaml")
  FileUtils.cp(source, clean)
  output, status = Open3.capture2e("ruby", validator, clean)
  abort "positive control failed: #{output}" unless status.success?

  mutations = {
    "shared assertion" => ["bad assertions", ->(doc) { doc["scenarios"][0]["assertions"].pop }],
    "eager flag" => ["lightr case lacks --eager", ->(doc) { doc["scenarios"][0]["lightr"].delete("--eager") }],
    "profile" => ["profile not activated", ->(doc) { doc["scenarios"][0]["docker"].delete("default") }],
    "engine" => ["bad engine", ->(doc) { doc["scenarios"][0]["lightr_engine"] = "bad" }],
    "schema" => ["bad case", ->(doc) { doc["scenarios"][0]["readiness"] = [] }]
  }
  mutations.each do |name, (expected, mutate)|
    doc = YAML.safe_load(File.read(source), aliases: false)
    mutate.call(doc)
    File.write(clean, YAML.dump(doc))
    reject!(validator, clean, name, expected)
  end
end

puts "s3 eager teeth: PASS"
