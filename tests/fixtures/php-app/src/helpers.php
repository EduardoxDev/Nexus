<?php

function money(int $cents): string
{
    return number_format($cents / 100, 2);
}

const APP_NAME = 'shop';
