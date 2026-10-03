# frozen_string_literal: true

require "test_helper"

class GemspecTest < Minitest::Test
  GEMSPEC = File.join(KitTest::RUBY_DIR, "lingara-apps.gemspec")

  # 30.9.26am AC27: the gemspec has exactly one runtime dependency, the
  # lingara library, from LIBRARY_FLOOR in RubyGems' spelling to below 0.2;
  # a 3.3 floor; and require_paths and bindir left unset, so the publisher
  # reads it without running it.
  def test_the_only_runtime_dependency_is_the_library
    spec = Gem::Specification.load(GEMSPEC)
    assert_equal "lingara-apps", spec.name
    assert_equal Lingara::Apps::GEM_VERSION, spec.version.to_s
    assert_equal ["lingara"], spec.runtime_dependencies.map(&:name)
    floor = File.read(File.join(KitTest::ROOT, "LIBRARY_FLOOR")).strip.sub("-", ".pre.")
    assert_equal Gem::Requirement.new(">= #{floor}", "< 0.2"), spec.runtime_dependencies.first.requirement
    assert_equal Gem::Requirement.new(">= 3.3"), spec.required_ruby_version
    assert_equal ["lib"], spec.require_paths
    text = File.read(GEMSPEC)
    refute_match(/^\s*[^#\n]*\brequire_paths\s*=/, text)
    refute_match(/^\s*[^#\n]*\bbindir\s*=/, text)
    assert_includes spec.files, "lib/lingara/apps.rb"
    assert_includes spec.files, "lib/lingara/apps/generated/card.rb"
    refute(spec.files.any? { |f| f.start_with?("test/", "conformance/", "codegen/") })
  end
end
