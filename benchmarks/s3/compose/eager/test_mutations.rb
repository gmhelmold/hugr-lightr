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
    "shared assertion" => ["bad assertions", ->(doc) { doc["cases"][0]["assertions"].pop }],
    "eager flag" => ["lightr case lacks --eager", ->(doc) { doc["cases"][0]["lightr"].delete("--eager") }],
    "namespace" => ["bad resources", ->(doc) { doc["cases"][0]["resources"][0] = "global-web" }],
    "readiness" => ["bad readiness", ->(doc) { doc["cases"][0]["readiness"] = [] }],
    "teardown" => ["bad teardown", ->(doc) { doc["cases"][0]["teardown"].delete("lightr") }]
  }
  mutations.each do |name, (expected, mutate)|
    doc = YAML.safe_load(File.read(source), aliases: false)
    mutate.call(doc)
    File.write(clean, YAML.dump(doc))
    reject!(validator, clean, name, expected)
  end
end

puts "s3 eager teeth: PASS"
