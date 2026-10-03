<?php

declare(strict_types=1);

namespace Lingara\Apps;

use Psr\Http\Message\ResponseFactoryInterface;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestInterface;
use Psr\Http\Message\StreamFactoryInterface;
use Psr\Http\Message\StreamInterface;
use Psr\Http\Server\RequestHandlerInterface;

/**
 * The PSR-15 adapter (ADR 30.9.26am D2): a request handler over any PSR-7
 * implementation, answering with the PSR-17 factories it is given. Mount it
 * on the route Lingara posts to; it reads the raw body, verifies, dispatches
 * and replies.
 *
 *     $factory = new Nyholm\Psr7\Factory\Psr17Factory();
 *     $handler = new Psr15Handler($app, $factory, $factory);
 */
final class Psr15Handler implements RequestHandlerInterface
{
    public function __construct(
        private readonly App $app,
        private readonly ResponseFactoryInterface $responses,
        private readonly StreamFactoryInterface $streams,
    ) {}

    public function handle(ServerRequestInterface $request): ResponseInterface
    {
        $answer = Core::handle(
            $this->app,
            $request->getMethod(),
            $request,
            static fn(int $limit): string => self::read($request->getBody(), $limit),
        );
        $response = $this->responses->createResponse($answer->status);
        foreach ($answer->headers as $name => $value) {
            $response = $response->withHeader($name, $value);
        }
        return $response->withBody($this->streams->createStream($answer->body));
    }

    /**
     * At most `$limit` bytes of the raw body, from its start: a middleware
     * that read it first (to parse JSON) leaves a seekable stream at its end.
     */
    private static function read(StreamInterface $body, int $limit): string
    {
        if ($body->isSeekable()) {
            $body->rewind();
        }
        $bytes = '';
        while (strlen($bytes) < $limit && !$body->eof()) {
            $chunk = $body->read($limit - strlen($bytes));
            if ($chunk === '') {
                break;
            }
            $bytes .= $chunk;
        }
        return $bytes;
    }
}
