# frozen_string_literal: true

require_relative "lib/framequery/version"

Gem::Specification.new do |s|
  s.name        = "framequery"
  s.version     = FrameQuery::VERSION
  s.summary     = "Ruby SDK for the FrameQuery API"
  s.description = "Upload videos, poll for job completion, and fetch scene/transcript " \
                  "results from the FrameQuery API. Uses net/http, zero runtime deps."
  s.authors     = ["FrameQuery"]
  s.email       = ["sdk@framequery.com"]
  s.homepage    = "https://github.com/framequery/framequery-sdks"
  s.license     = "MIT"

  s.required_ruby_version = ">= 3.0"

  s.files = Dir["lib/**/*.rb"] + ["README.md", "LICENSE"]

  s.add_development_dependency "minitest", "~> 5.0"
  s.add_development_dependency "webmock", "~> 3.0"
  s.add_development_dependency "rake", "~> 13.0"
end
