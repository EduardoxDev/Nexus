package com.acme.shop

import com.acme.shop.cart.Cart
import com.acme.shop.util.formatMoney
import kotlin.math.max

fun main() {
    val cart = Cart()
    println(formatMoney(max(cart.total(), 0)))
}
