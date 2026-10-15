package com.acme.shop.cart

import com.acme.shop.util.*

const val MAX_ITEMS = 100

class Cart {
    private val items = mutableListOf<Item>()

    fun add(item: Item) {
        require(items.size < MAX_ITEMS)
        items.add(item)
    }

    fun total(): Long = items.sumOf { it.price }

    companion object {
        fun empty() = Cart()
    }
}
