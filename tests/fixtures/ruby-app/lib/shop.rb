require "json"
require "shop/cart"
require_relative "shop/version"

module Shop
  def self.run
    Cart.new.total
  end
end
