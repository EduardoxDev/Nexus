require "minitest/autorun"
require "shop/cart"

class CartTest < Minitest::Test
  def test_empty_total
    assert_equal 0, Shop::Cart.new.total
  end
end
