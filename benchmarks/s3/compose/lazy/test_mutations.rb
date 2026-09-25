#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "open3"
require "tmpdir"

root = File.expand_path(__dir__)
source = File.join(root, "receipt.json")
validator = File.join(root, "validate.rb")
Dir.mktmpdir("s3-lazy-teeth") do |dir|
  path = File.join(dir, "receipt.json")
  clean, status = Open3.capture2e("ruby", validator, source)
  abort "positive control failed: #{clean}" unless status.success?
  { "authority" => ["bad receipt fields", ->(d) { d["release_token"] = "x" }], "pid" => ["bad pid", ->(d) { d["pid"] = nil }], "order" => ["bad timestamps", ->(d) { d["resume_at_unix_ms"] = 0 }] }.each do |name, (message, mutate)|
    doc = JSON.parse(File.read(source)); mutate.call(doc); File.write(path, JSON.generate(doc))
    output, result = Open3.capture2e("ruby", validator, path)
    abort "#{name} mutation passed" if result.success?
    abort "#{name} wrong failure: #{output}" unless output.include?(message)
  end
end
puts "s3 lazy teeth: PASS"
