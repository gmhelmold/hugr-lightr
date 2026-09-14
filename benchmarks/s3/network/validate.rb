#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"

def fail!(reason)
  warn "s3 network: #{reason}"
  exit 1
end

path = ARGV.fetch(0) { fail!("contract path required") }
doc = YAML.safe_load(File.read(path), aliases: false)
fail!("bad document") unless doc.is_a?(Hash) && doc.keys.sort == %w[cases schema_version] && doc["schema_version"] == 1
cases = doc["cases"]
fail!("bad cases") unless cases.is_a?(Array) && cases.length == 3
by_id = cases.each_with_object({}) { |entry, out| out[entry["id"]] = entry if entry.is_a?(Hash) }
fail!("missing case") unless by_id.keys.sort == %w[hotplug-refusal registry-lifecycle spawn-membership-dns-alias]

registry = by_id["registry-lifecycle"]
fail!("bad registry lifecycle") unless registry == {
  "id" => "registry-lifecycle",
  "test" => "network::registry::tests::s3_network_registry_remove_refuses_member_then_removes_empty_network",
  "outcome" => "locked-remove-refuses-live-member"
}

spawn = by_id["spawn-membership-dns-alias"]
fail!("bad spawn DNS") unless spawn == {
  "id" => "spawn-membership-dns-alias",
  "test" => "vswitch::switch_host_tests::attach_forward_dhcp_dns_then_refcount_self_stop",
  "outcome" => "alias-resolves-to-registry-ip"
}

hotplug = by_id["hotplug-refusal"]
fail!("bad hotplug refusal") unless hotplug == {
  "id" => "hotplug-refusal",
  "test" => "handlers::network::tests::s3_network_connect_is_typed_refusal_exit_2",
  "error_class" => "InvalidRef",
  "exit_code" => 2,
  "message" => "network connect/disconnect unsupported: set --network when creating run"
}

puts "s3 network: PASS"
