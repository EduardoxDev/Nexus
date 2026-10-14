<?php
namespace App\Support;

interface Clock
{
    public function now(): int;
}
