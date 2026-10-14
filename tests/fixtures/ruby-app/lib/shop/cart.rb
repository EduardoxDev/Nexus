require_relative "item"

module Shop
  class Cart
    MAX_ITEMS = 100

    def initialize
      @items = []
    end

    def add(item)
      @items << item if @items.size < MAX_ITEMS
    end

    def total
      @items.sum(&:price)
    end

    private

    def clear!
      @items = []
    end
  end
end
