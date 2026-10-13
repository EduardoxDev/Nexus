package com.acme.shop.util;

import java.util.Locale;

public final class Money {
    private Money() {
    }

    public static String format(long cents) {
        return String.format(Locale.ROOT, "R$ %d,%02d", cents / 100, cents % 100);
    }
}
