<?php

declare(strict_types=1);

namespace Lingara\Apps\Tests;

use PHPUnit\Framework\TestCase;

final class ComposerTest extends TestCase
{
    /**
     * 30.9.26am AC28: composer.json's `require` is php, the library (at the
     * floor in LIBRARY_FLOOR, as a caret) and the two PSR interface packages
     * only, and it has no `version` field.
     */
    public function testRequireIsTheLibraryAndTheHandlerInterfaces(): void
    {
        $manifest = json_decode((string) file_get_contents(__DIR__ . '/../composer.json'), true, 512, JSON_THROW_ON_ERROR);
        self::assertIsArray($manifest);
        self::assertIsArray($manifest['require']);
        $require = array_keys($manifest['require']);
        sort($require);
        self::assertSame(['php', 'psr/http-factory', 'psr/http-server-handler', 'spinningcatstudios/lingara'], $require);
        $floor = trim((string) file_get_contents(__DIR__ . '/../../LIBRARY_FLOOR'));
        self::assertSame("^{$floor}", $manifest['require']['spinningcatstudios/lingara']);
        self::assertSame('>=8.2', $manifest['require']['php']);
        self::assertArrayNotHasKey('version', $manifest);
        self::assertSame('spinningcatstudios/lingara-apps', $manifest['name']);
        self::assertSame('MIT', $manifest['license']);
    }
}
