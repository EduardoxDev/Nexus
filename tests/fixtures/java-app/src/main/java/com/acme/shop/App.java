package com.acme.shop;

import java.util.List;
import com.acme.shop.cart.Cart;
import static com.acme.shop.util.Money.format;

/** Ponto de entrada: class Fake {} não é declaração. */
public class App {
    public static void main(String[] args) {
        Cart cart = new Cart();
        System.out.println(format(cart.total()));
    }
}
