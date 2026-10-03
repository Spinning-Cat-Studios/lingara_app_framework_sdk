# frozen_string_literal: true

require "open3"
require "rbconfig"
require "tmpdir"
require "test_helper"

class CodegenTest < Minitest::Test
  SCRIPT = File.join(KitTest::RUBY_DIR, "codegen/generate.rb")
  VIEW = File.join(KitTest::ROOT, "spec/generator/apps.3.0.json")
  VERSION_FILE = File.join(KitTest::ROOT, "VERSION")

  ARMS = {
    "CardElement" => %w[CardElementHeading CardElementText CardElementTerm CardElementList CardElementProgress
      CardElementDivider CardElementButton CardElementLink],
    "ListItem" => %w[ListItemText ListItemTerm],
    "ContextSlice" => %w[ContextSliceLanguages ContextSlicePlanSummary ContextSliceReviewDue ContextSliceTutorTopic]
  }.freeze

  def generate(out, models)
    Open3.capture3(RbConfig.ruby, SCRIPT, "--view", VIEW, "--version", VERSION_FILE, "--out", out, "--models", models)
  end

  # 30.9.26am AC20: over the committed view, generate.rb writes CardElement,
  # ListItem and ContextSlice with S1 D4's arm names, the four slice kinds
  # and the app.render / app.action operations with the app.card reply; two
  # runs are byte-identical; a models directory holding a file named after a
  # union or an arm is refused; and the core's dispatch keys equal the
  # generated operations.
  def test_the_emitter_writes_the_three_unions_and_the_operations
    Dir.mktmpdir do |dir|
      out = File.join(dir, "out")
      models = File.join(dir, "models")
      Dir.mkdir(models)
      _, err, status = generate(out, models)
      assert status.success?, err

      unions = File.read(File.join(out, "unions.rb"))
      assert_equal ARMS.keys.sort, unions.scan(/^    module (\w+)$/).flatten.uniq.sort - ["Unions"]
      ARMS.each do |union, arms|
        assert_equal arms, unions.scan(/^    (\w+) = Data\.define.*\n      include #{union}$/).flatten, union
      end
      assert_includes unions, %(CONTEXT_SLICE_KINDS = ["languages", "plan_summary", "review_due", "tutor_topic"].freeze)
      assert_includes unions, %("list" => CardElementList)

      operations = File.read(File.join(out, "operations.rb"))
      assert_includes operations, %(RENDER = "app.render")
      assert_includes operations, %(ACTION = "app.action")
      assert_includes operations, %(REPLY = "app.card")

      first = %w[unions.rb operations.rb version.rb].to_h { |f| [f, File.binread(File.join(out, f))] }
      generate(out, models)
      first.each { |f, bytes| assert_equal bytes, File.binread(File.join(out, f)), "#{f} differs between runs" }

      %w[card_element.rb context_slice_languages.rb].each do |name|
        File.write(File.join(models, name), "")
        _, err, status = generate(out, models)
        refute status.success?, name
        assert_includes err, name
        File.delete(File.join(models, name))
      end
    end

    # The committed output, loaded: each arm includes its union, and the core
    # dispatches exactly the generated request messages.
    ARMS.each do |union, arms|
      mod = Lingara::Apps.const_get(union)
      assert_equal arms, mod::ARMS.values.map { |arm| arm.name.delete_prefix("Lingara::Apps::") }
      assert(mod::ARMS.values.all? { |arm| arm.include?(mod) })
    end
    assert_equal %w[app.action app.render], Lingara::Apps::Operations::REQUESTS.keys.sort
    assert_equal Lingara::Apps::Operations::REQUESTS.keys.sort, Lingara::Apps::App::DISPATCH.keys.sort
    assert_equal Lingara::Apps::CONTEXT_SLICE_KINDS, Lingara::Apps::ContextSlice::ARMS.keys
  end
end
