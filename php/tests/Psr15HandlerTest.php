<?php

declare(strict_types=1);

namespace Lingara\Apps\Tests;

use GuzzleHttp\Psr7\HttpFactory;
use Lingara\Apps\App;
use Lingara\Apps\Card;
use Lingara\Apps\Generated\AppActionRequest;
use Lingara\Apps\Generated\AppRenderRequest;
use Lingara\Apps\Generated\AppSlotName;
use Lingara\Apps\Generated\Card as CardModel;
use Lingara\Apps\Generated\ContextSliceLanguages;
use Lingara\Apps\Generated\ContextSlicePlanSummary;
use Lingara\Apps\Psr15Handler;
use Nyholm\Psr7\Factory\Psr17Factory;
use PHPUnit\Framework\TestCase;
use Psr\Http\Message\ResponseFactoryInterface;
use Psr\Http\Message\ResponseInterface;
use Psr\Http\Message\ServerRequestFactoryInterface;
use Psr\Http\Message\StreamFactoryInterface;

final class Psr15HandlerTest extends TestCase
{
    private const KEY = 'psr15-handler-test-key-32-bytes!';
    private const ETAG = 'c1_Zx/9+=é…"\\ ';

    private string $log;
    private string|false $previousLog;

    protected function setUp(): void
    {
        $this->log = (string) tempnam(sys_get_temp_dir(), 'lgr-apps-log');
        $this->previousLog = ini_set('error_log', $this->log);
    }

    protected function tearDown(): void
    {
        ini_set('error_log', (string) $this->previousLog);
        @unlink($this->log);
    }

    /**
     * 30.9.26am AC14: through Psr15Handler, on both PSR-7 implementations, a
     * tampered body is `401` and never reaches the render function; a valid
     * render reaches it with the decoded `subject`, slot and slices, an
     * unknown slice `kind` skipped; an action's `card_etag` reaches its
     * function byte for byte.
     */
    public function testVerifiesTheRawBodyBeforeDispatch(): void
    {
        foreach ([new Psr17Factory(), new HttpFactory()] as $factory) {
            $seen = new Seen();
            $app = new App(
                secret: 'lgr_whsec_' . base64_encode(self::KEY),
                render: static function (AppRenderRequest $request) use ($seen): CardModel {
                    $seen->renders[] = $request;
                    return Card::create()->heading('hello', 1)->build();
                },
                actions: ['inc' => static function (AppActionRequest $request) use ($seen): CardModel {
                    $seen->etags[] = $request->getCardEtag();
                    return Card::create()->text('done')->build();
                }],
            );
            $handler = new Psr15Handler($app, $factory, $factory);
            $render = self::body('app.render', []);

            $tampered = self::send($handler, $factory, str_replace('lgr_sub_learner', 'lgr_sub_lEarner', $render), self::sign($render));
            self::assertSame(401, $tampered->getStatusCode());
            self::assertSame('', (string) $tampered->getBody());
            self::assertSame(0, $seen->renderCount(), 'a tampered body never reaches the render function');

            $ok = self::send($handler, $factory, $render, self::sign($render));
            self::assertSame(200, $ok->getStatusCode(), (string) $ok->getBody());
            self::assertSame('application/json', $ok->getHeaderLine('content-type'));
            self::assertSame('{"card":{"elements":[{"type":"heading","text":"hello","level":1}]}}', (string) $ok->getBody());
            self::assertSame(1, $seen->renderCount());
            $request = $seen->firstRender();
            self::assertSame('lgr_sub_learner', $request->getSubject());
            self::assertSame(AppSlotName::HOME_SIDE, $request->getSlot());
            $context = $request->getContext();
            self::assertCount(2, $context, 'the unknown slice kind is skipped');
            self::assertInstanceOf(ContextSliceLanguages::class, $context[0]);
            self::assertSame('zh', $context[0]->targetLang);
            self::assertInstanceOf(ContextSlicePlanSummary::class, $context[1]);
            self::assertSame(3, $context[1]->setsCompleted);
            self::assertStringContainsString('unknown kind "weather"', (string) file_get_contents($this->log));

            $action = self::body('app.action', ['action_id' => 'inc', 'card_etag' => self::ETAG]);
            $pressed = self::send($handler, $factory, $action, self::sign($action));
            self::assertSame(200, $pressed->getStatusCode(), (string) $pressed->getBody());
            self::assertSame([self::ETAG], $seen->etags, 'card_etag reaches the action byte for byte');
        }
    }

    /** @param array<string, string> $extra */
    private static function body(string $type, array $extra): string
    {
        return json_encode([
            'type' => $type,
            'id' => 'lgr_msg_' . str_repeat('ab', 16),
            'install_id' => '7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f',
            'subject' => 'lgr_sub_learner',
            'slot' => 'home.side',
            'locale' => 'en',
            'unknown_member' => ['ignored' => true],
            'context' => [
                ['kind' => 'languages', 'source_lang' => 'en', 'target_lang' => 'zh', 'level' => 2, 'extra' => 1],
                ['kind' => 'weather', 'sky' => 'grey'],
                ['kind' => 'plan_summary', 'plan_id' => '0d9e8f7a-6b5c-4d3e-8f2a-1b0c9d8e7f6a', 'title' => 'Market',
                    'target_lang' => 'zh', 'set_count' => 5, 'sets_completed' => 3],
            ],
            ...$extra,
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES);
    }

    /**
     * Standard Webhooks, as Lingara signs: HMAC-SHA256 over id.timestamp.body.
     *
     * @return array<string, string>
     */
    private static function sign(string $body): array
    {
        $id = 'lgr_msg_' . str_repeat('ab', 16);
        $timestamp = (string) time();
        $signature = base64_encode(hash_hmac('sha256', "{$id}.{$timestamp}.{$body}", self::KEY, true));
        return ['webhook-id' => $id, 'webhook-timestamp' => $timestamp, 'webhook-signature' => "v1,{$signature}"];
    }

    /** @param array<string, string> $headers */
    private static function send(
        Psr15Handler $handler,
        ServerRequestFactoryInterface&StreamFactoryInterface&ResponseFactoryInterface $factory,
        string $body,
        array $headers,
    ): ResponseInterface {
        $request = $factory->createServerRequest('POST', 'http://127.0.0.1/lingara')
            ->withHeader('content-type', 'application/json')
            ->withBody($factory->createStream($body));
        foreach ($headers as $name => $value) {
            $request = $request->withHeader($name, $value);
        }
        return $handler->handle($request);
    }
}

/** What the app's functions saw. */
final class Seen
{
    /** @var list<AppRenderRequest> */
    public array $renders = [];
    /** @var list<string> */
    public array $etags = [];

    public function renderCount(): int
    {
        return count($this->renders);
    }

    public function firstRender(): AppRenderRequest
    {
        return $this->renders[0] ?? throw new \LogicException('the render function was never called');
    }
}
