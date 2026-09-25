#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"

def fail!(reason)
  warn "s3 lazy: #{reason}"
  exit 1
end

path = ARGV.fetch(0) { fail!("receipt path required") }
raw = File.binread(path).delete_suffix("\n")
fail!("receipt is not JCS") unless raw == JSON.generate(JSON.parse(raw)).then { |s| JSON.generate(JSON.parse(s)).force_encoding(Encoding::BINARY) }
doc = JSON.parse(raw)
fields = %w[accept_at_unix_ms artifact_sha256 first_byte_at_unix_ms instance_id pid resume_at_unix_ms schema service]
fail!("bad receipt fields") unless doc.keys.sort == fields
fail!("bad schema") unless doc["schema"] == "lazy_v1"
fail!("bad identity") unless doc["instance_id"].is_a?(String) && !doc["instance_id"].empty? && doc["artifact_sha256"].match?(/\A[0-9a-f]{64}\z/)
fail!("bad pid") unless doc["pid"].is_a?(Integer) && doc["pid"] > 0
fail!("bad timestamps") unless %w[accept_at_unix_ms first_byte_at_unix_ms resume_at_unix_ms].all? { |k| doc[k].is_a?(Integer) } && doc["accept_at_unix_ms"] <= doc["first_byte_at_unix_ms"] && doc["first_byte_at_unix_ms"] <= doc["resume_at_unix_ms"]
fail!("authority leaked") if raw.include?("release_token")
puts "s3 lazy: PASS"
