package com.acme.shop.cart;

import java.util.ArrayList;
import java.util.List;

public class Cart {
    public static final int MAX_ITEMS = 100;
    private final List<Item> items = new ArrayList<>();

    public Cart() {
    }

    public void add(Item item) {
        if (items.size() >= MAX_ITEMS) {
            throw new IllegalStateException("carrinho cheio");
        }
        items.add(item);
    }

    @Override
    public String toString() {
        return "Cart{" + items.size() + "}";
    }

    public long total() {
        return items.stream().mapToLong(Item::priceCents).sum();
    }
}
