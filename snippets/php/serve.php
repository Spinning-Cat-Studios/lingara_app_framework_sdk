<?php

declare(strict_types=1);

namespace LingaraAppsSnippets;

use Lingara\Apps\App;
// lingara:begin serve
use Lingara\Apps\Psr15Handler;
use Nyholm\Psr7\Factory\Psr17Factory;
use Nyholm\Psr7Server\ServerRequestCreator;
// lingara:end

function serve(App $app): void
{
    // lingara:begin serve
    // A PSR-15 handler: mount it on your framework's route, or answer from a
    // front controller as here. It reads the raw body, verifies, dispatches and replies.
    $factory = new Psr17Factory();
    $handler = new Psr15Handler($app, $factory, $factory);
    $response = $handler->handle((new ServerRequestCreator($factory, $factory, $factory, $factory))->fromGlobals());
    http_response_code($response->getStatusCode());
    foreach ($response->getHeaders() as $name => $values) {
        header("{$name}: " . implode(', ', $values));
    }
    echo $response->getBody();
    // lingara:end
}
