<?php

declare(strict_types=1);

namespace Lingara\Apps\Tests;

use Lingara\Apps\Card;
use Lingara\Apps\CardLimitException;
use Lingara\Apps\Limits;
use Lingara\Apps\Manifest;
use Lingara\Apps\ManifestException;
use PHPUnit\Framework\TestCase;

final class VectorsTest extends TestCase
{
    private const VECTORS = __DIR__ . '/../../conformance/vectors';

    /**
     * 30.9.26am AC7: every card-limits.json vector's `expect` equals
     * validateReply's answer, every manifest.json vector's equals
     * validateManifest's, and truncate of an 81-scalar astral heading at 80
     * is 79 scalars plus `…`.
     */
    public function testEveryCardAndManifestVectorGivesItsExpectedAnswer(): void
    {
        $cards = self::vectors('card-limits.json');
        self::assertGreaterThan(70, count($cards));
        foreach ($cards as $vector) {
            self::assertEquals($vector->expect, self::replyAnswer($vector->reply), $vector->name);
        }

        $manifests = self::vectors('manifest.json');
        self::assertGreaterThan(40, count($manifests));
        foreach ($manifests as $vector) {
            self::assertEquals($vector->expect, self::manifestAnswer($vector->manifest), $vector->name);
        }

        $heading = str_repeat('𝄞', 81);
        $cut = Limits::truncate($heading, 80);
        self::assertSame(80, mb_strlen($cut, 'UTF-8'));
        self::assertSame(str_repeat('𝄞', 79) . '…', $cut);
        self::assertSame(Limits::truncate(str_repeat('𝄞', 80), 80), str_repeat('𝄞', 80), 'at the limit, unchanged');
        Card::create()->heading($cut, 1)->build();
    }

    private static function replyAnswer(mixed $reply): \stdClass
    {
        try {
            Limits::validateReply($reply);
            return (object) ['ok' => true];
        } catch (CardLimitException $e) {
            return (object) ['refused' => $e->reason];
        }
    }

    private static function manifestAnswer(mixed $manifest): \stdClass
    {
        try {
            Manifest::validateManifest($manifest);
            return (object) ['ok' => true];
        } catch (ManifestException $e) {
            return (object) ['refused' => $e->rule];
        }
    }

    /** @return list<object{name: string, expect: \stdClass, reply: mixed, manifest: mixed}> */
    private static function vectors(string $file): array
    {
        $doc = json_decode((string) file_get_contents(self::VECTORS . "/{$file}"), false, 512, JSON_THROW_ON_ERROR);
        self::assertInstanceOf(\stdClass::class, $doc);
        self::assertIsArray($doc->vectors);
        /** @var list<object{name: string, expect: \stdClass, reply: mixed, manifest: mixed}> */
        return $doc->vectors;
    }
}
