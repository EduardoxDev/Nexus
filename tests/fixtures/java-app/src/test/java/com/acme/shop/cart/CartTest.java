package com.acme.shop.cart;

import com.acme.shop.util.*;
import org.junit.jupiter.api.Test;

class CartTest {
    @Test
    void addsItems() {
        Cart cart = new Cart();
        cart.add(new Item("a1", 1000));
        Money.format(cart.total());
    }
}
