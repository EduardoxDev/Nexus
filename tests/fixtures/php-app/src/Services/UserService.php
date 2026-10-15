<?php

namespace App\Services;

use App\Models\User;
use App\Support\{Clock, Logger};
use Psr\Log\LoggerInterface;

require_once __DIR__ . '/../helpers.php';

final class UserService
{
    private const CACHE_TTL = 60;

    public function create(string $name): User
    {
        return new User($name);
    }

    private function log(string $msg): void {}
}
