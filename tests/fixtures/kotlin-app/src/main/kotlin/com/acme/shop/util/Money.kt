package com.acme.shop.util

typealias Cents = Long

fun formatMoney(cents: Long): String = "R$ %.2f".format(cents / 100.0)

private fun round(v: Double) = Math.round(v)
