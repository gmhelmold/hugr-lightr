#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"

source = YAML.load_file(ARGV.fetch(0))
cli_commit = "b85ea310f6d5ea4fcf843f37edaa26ab2ac5b3af"
categories = %w[build buildkit run compose network volume security resources logging registry health plugin swarm]
metrics = %w[wall_clock_ms cpu_user_ms cpu_sys_ms rss_peak_bytes]

reasons = {
  "plugin" => "Plugin ecosystem is out_of_scope in Sprint 1 (SPRINT-01-EXECUTION-PLAN.md:70); no executable fixture is declared.",
  "swarm" => "Swarm enters planned Sprint 4 (docs/spec/feature-parity.md:81-84); no verified fixture or command exists.",
  "buildkit" => "No verified BuildKit fixture path/context or exact command exists at declared source commit.",
  "compose" => "No verified compose fixture path/context or exact command exists at declared source commit.",
  "registry" => "No verified registry fixture path/context or exact command exists at declared source commit.",
  "network" => "No verified network fixture path/context or exact command exists at declared source commit.",
  "volume" => "No verified volume fixture path/context or exact command exists at declared source commit.",
  "security" => "No verified security fixture path/context or exact command exists at declared source commit.",
  "resources" => "No verified resource-limit fixture path/context or exact command exists at declared source commit.",
  "health" => "No verified health fixture path/context or exact command exists at declared source commit.",
  "logging" => "No verified logging fixture path/context or exact command exists at declared source commit.",
  "build" => "No verified build fixture path/context or exact command exists at declared source commit.",
  "run" => "No verified run fixture path/context or exact command exists at declared source commit."
}.freeze

hardware_ids = source.fetch("scenarios").select { |scenario| scenario["honest_gated"] || scenario["availability"] == "hardware_gated" }.map { |scenario| scenario.fetch("id") }.to_h { |id| [id, true] }

scenarios = source.fetch("scenarios").map do |scenario|
  category = scenario.fetch("category")
  project = scenario["project"] || scenario.dig("fixture", "project")
  availability = if category == "plugin"
                   "out_of_scope"
                 elsif hardware_ids.key?(scenario.fetch("id"))
                   "hardware_gated"
                 else
                   "unsupported"
                 end
  reason = reasons.fetch(category)
  reason = "Host capability required and no verified fixture or exact command exists at declared source commit." if availability == "hardware_gated"

  {
    "id" => scenario.fetch("id"),
    "category" => category,
    "availability" => availability,
    "reason" => reason,
    "fixture" => {
      "project" => project,
      "path" => nil,
      "context" => nil
    },
    "docker" => { "command" => nil },
    "lightr" => { "command" => nil },
    "metrics" => metrics,
    "tags" => [category, "unverified-fixture"]
  }
end

output = {
  "metadata" => {
    "spec_version" => "1.0.0-s1-a-frozen",
    "target" => "Docker CLI/Engine 28.3.2",
    "source_commit" => cli_commit,
    "status" => "evidence-gated"
  },
  "taxonomy" => {
    "category" => categories,
    "evidence" => ["unverified-fixture"]
  },
  "source_evidence" => {
    "lightr_cli" => {
      "commit" => cli_commit,
      "files" => [
        "crates/lightr-cli/src/cli/cmd/mod.rs",
        "crates/lightr-cli/src/cli/cmd/subcommands.rs"
      ],
      "observed_surface" => ["build", "compose", "network", "oci", "run", "volume"]
    },
    "projects" => [
      {
        "id" => "kubernetes-kind",
        "repo" => "https://github.com/kubernetes/kubernetes.git",
        "tag" => "v1.29.4",
        "raw_tag_object" => "5d985937e8a112a321916efd4ad3936c7db6345f",
        "peeled_commit" => "55019c83b0fd51ef4ced8c29eec2c4847f896e74"
      },
      {
        "id" => "elasticsearch",
        "repo" => "https://github.com/elastic/elasticsearch.git",
        "tag" => "v8.13.4",
        "raw_tag_object" => "85ff3fe65dcf2ab0185083aee4b8f462a92ab289",
        "peeled_commit" => "da95df118650b55a500dcc181889ac35c6d8da7c"
      }
    ]
  },
  "scenarios" => scenarios
}

File.write(ARGV.fetch(1), YAML.dump(output).gsub(/: \n/, ": null\n"))
