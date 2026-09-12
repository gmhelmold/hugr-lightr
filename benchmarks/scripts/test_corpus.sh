#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
spec=${1:-"$root/benchmarks/benchmark-spec.yaml"}

ruby -r yaml -r open3 - "$root" "$spec" <<'RUBY'
root = ARGV.fetch(0)
spec_path = ARGV.fetch(1)
spec = YAML.load_file(spec_path)
errors = []
scenarios = spec["scenarios"]
unless scenarios.is_a?(Array)
  abort "FAIL: scenarios must be an array"
end

ids = scenarios.map { |scenario| scenario["id"] }
errors << "expected 250 scenarios, got #{scenarios.length}" unless scenarios.length == 250
errors << "scenario IDs must be present and unique" unless ids.all? { |id| id.is_a?(String) && !id.empty? } && ids.uniq.length == 250

taxonomy = spec.fetch("taxonomy", {})
tags = taxonomy.values.flatten
categories = taxonomy.fetch("category", [])
states = %w[supported unsupported hardware_gated out_of_scope]
counts = Hash.new(0)
assertion_kinds = %w[exit_code stdout_exact stdout_regex stderr_regex file_sha256 http_status command]
generic = /\|\||\bDockerfile\b|<[^>]+>|:bench\b|-(?:add|additional)-\d+/

scenarios.each do |scenario|
  id = scenario["id"] || "<missing-id>"
  category = scenario["category"]
  availability = scenario["availability"]
  counts[availability] += 1
  errors << "#{id}: invalid category" unless categories.include?(category)
  errors << "#{id}: invalid availability" unless states.include?(availability)
  errors << "#{id}: reason required" if availability != "supported" && !(scenario["reason"].is_a?(String) && !scenario["reason"].empty?)
  fixture = scenario["fixture"]
  errors << "#{id}: fixture project/path/context required" unless fixture.is_a?(Hash) && fixture.key?("project") && fixture.key?("path") && fixture.key?("context")
  errors << "#{id}: four metrics required" unless scenario["metrics"].is_a?(Array) && scenario["metrics"].length >= 4
  errors << "#{id}: two tags required" unless scenario["tags"].is_a?(Array) && scenario["tags"].length >= 2
  Array(scenario["tags"]).each { |tag| errors << "#{id}: unregistered tag #{tag}" unless tags.include?(tag) }
  docker = scenario.dig("docker", "command")
  lightr = scenario.dig("lightr", "command")
  errors << "#{id}: generic command" if [docker, lightr].compact.any? { |command| command.match?(generic) }
  if availability == "supported"
    errors << "#{id}: supported requires fixture path/context" unless fixture["path"].is_a?(String) && fixture["context"].is_a?(String)
    errors << "#{id}: supported requires both commands" unless docker.is_a?(String) && lightr.is_a?(String)
    assertions = scenario["assertions"]
    errors << "#{id}: supported requires structured assertion" unless assertions.is_a?(Array) && assertions.any? { |assertion| assertion_kinds.include?(assertion["kind"]) }
  else
    errors << "#{id}: unsupported command must be null" unless docker.nil? && lightr.nil?
  end
end
errors << "conservation failed: #{counts}" unless counts.values.sum == 250 && states.sum { |state| counts[state] } == 250

projects = spec.dig("source_evidence", "projects")
project_index = {}
unless projects.is_a?(Array) && projects.length.positive?
  errors << "source evidence projects missing"
else
  projects.each do |project|
    %w[id repo tag raw_tag_object peeled_commit].each { |field| errors << "source evidence missing #{field}" unless project[field].is_a?(String) && !project[field].empty? }
    next unless project["repo"] && project["tag"]
    output, status = Open3.capture2e("git", "ls-remote", project["repo"], "refs/tags/#{project["tag"]}", "refs/tags/#{project["tag"]}^{}")
    if !status.success?
      errors << "#{project["id"]}: remote tag query failed"
      next
    end
    refs = output.lines.map { |line| hash, ref = line.strip.split(/\s+/, 2); [ref, hash] }.to_h
    raw = refs["refs/tags/#{project["tag"]}"]
    peeled = refs["refs/tags/#{project["tag"]}^{}"]
    errors << "#{project["id"]}: raw tag object mismatch" unless raw == project["raw_tag_object"]
    errors << "#{project["id"]}: peeled commit mismatch" unless peeled == project["peeled_commit"]
    project_index[project["id"]] = project
  end
end

cli = spec.dig("source_evidence", "lightr_cli")
if cli.is_a?(Hash)
  Array(cli["files"]).each do |file|
    _output, status = Open3.capture2e("git", "-C", root, "cat-file", "-e", "#{cli["commit"]}:#{file}")
    errors << "lightr CLI evidence file missing at source commit: #{file}" unless status.success?
  end
else
  errors << "lightr CLI source evidence missing"
end

scenarios.select { |scenario| scenario["availability"] == "supported" }.each do |scenario|
  id = scenario.fetch("id")
  fixture = scenario.fetch("fixture")
  next unless fixture["path"].is_a?(String)
  project = project_index[fixture["project"]]
  errors << "#{id}: fixture project lacks source evidence" and next unless project
  repo_path = project.fetch("repo").sub(%r{https://github.com/}, "").sub(/\.git\z/, "")
  url = "https://api.github.com/repos/#{repo_path}/contents/#{fixture.fetch("path")}?ref=#{project.fetch("peeled_commit")}"
  _output, status = Open3.capture2e("curl", "--fail", "--silent", "--show-error", url)
  errors << "#{id}: fixture path absent at peeled source commit" unless status.success?
  evidence = scenario["lightr_evidence"]
  unless evidence.is_a?(Hash) && evidence["source_file"].is_a?(String) && evidence["help_surface"].is_a?(String)
    errors << "#{id}: supported requires lightr source/help evidence"
    next
  end
  output, status = Open3.capture2e("git", "-C", root, "show", "#{cli["commit"]}:#{evidence["source_file"]}")
  errors << "#{id}: lightr source evidence missing" unless status.success? && output.include?(evidence["help_surface"])
end

if errors.empty?
  puts "PASS: 250 scenarios (supported=#{counts["supported"]}, unsupported=#{counts["unsupported"]}, hardware_gated=#{counts["hardware_gated"]}, out_of_scope=#{counts["out_of_scope"]})"
else
  errors.each { |error| warn "FAIL: #{error}" }
  exit 1
end
RUBY
