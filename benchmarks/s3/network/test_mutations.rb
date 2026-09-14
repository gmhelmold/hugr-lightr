#!/usr/bin/env ruby
# frozen_string_literal: true

require "open3"
require "tmpdir"
require "yaml"

source = File.join(__dir__, "contract.yaml")
VALIDATOR = File.join(__dir__, "validate.rb")

def validate!(path, expected)
  output, status = Open3.capture2e("ruby", VALIDATOR, path)
  abort "mutation wrong result: #{output}" unless status.success? == expected
end

Dir.mktmpdir("s3-network-teeth") do |dir|
  contract = File.join(dir, "contract.yaml")
  validate!(source, true)

  mutations = [
    ->(doc) { doc["cases"].find { |entry| entry["id"] == "hotplug-refusal" }["exit_code"] = 0 },
    ->(doc) { doc["cases"].find { |entry| entry["id"] == "hotplug-refusal" }["error_class"] = "Io" },
    ->(doc) { doc["cases"].find { |entry| entry["id"] == "spawn-membership-dns-alias" }["outcome"] = "no-alias" }
  ]
  mutations.each do |mutate|
    doc = YAML.safe_load(File.read(source), aliases: false)
    mutate.call(doc)
    File.write(contract, YAML.dump(doc))
    validate!(contract, false)
  end
end

puts "s3 network teeth: PASS"
