<?php

declare(strict_types=1);

namespace Lingara\Apps\Tests;

use Lingara\Apps\Core;
use Lingara\Apps\Generated\AppOperation;
use Lingara\Apps\Generated\CardElement;
use Lingara\Apps\Generated\ContextSlice;
use Lingara\Apps\Generated\ListItem;
use PHPUnit\Framework\TestCase;

final class CodegenTest extends TestCase
{
    private const SCRIPT = __DIR__ . '/../codegen/generate.php';
    private const VIEW = __DIR__ . '/../../spec/generator/apps.3.0.json';
    private const COMMITTED = __DIR__ . '/../src/Generated';

    private const UNIONS = [
        'CardElement' => ['heading' => 'CardElementHeading', 'text' => 'CardElementText', 'term' => 'CardElementTerm',
            'list' => 'CardElementList', 'progress' => 'CardElementProgress', 'divider' => 'CardElementDivider',
            'button' => 'CardElementButton', 'link' => 'CardElementLink'],
        'ListItem' => ['text' => 'ListItemText', 'term' => 'ListItemTerm'],
        'ContextSlice' => ['languages' => 'ContextSliceLanguages', 'plan_summary' => 'ContextSlicePlanSummary',
            'review_due' => 'ContextSliceReviewDue', 'tutor_topic' => 'ContextSliceTutorTopic'],
    ];

    /**
     * 30.9.26am AC21: over the committed view, generate.php writes
     * CardElement, ListItem and ContextSlice with S1 D4's arm names, each arm
     * implementing its union, the four slice kinds and the `app.render` /
     * `app.action` operations; two runs are byte-identical and equal the
     * committed files; it refuses a generator-written file named after a
     * union or an arm; and the core's dispatch keys equal the generated
     * operations.
     */
    public function testTheEmitterWritesTheThreeUnionsAndTheOperations(): void
    {
        $out = sys_get_temp_dir() . '/lgr-apps-codegen-' . bin2hex(random_bytes(4));
        [$status, $err] = self::generate($out);
        self::assertSame(0, $status, $err);
        $expected = ['AppOperation.php'];
        foreach (self::UNIONS as $union => $arms) {
            $expected[] = "{$union}.php";
            foreach ($arms as $arm) {
                $expected[] = "{$arm}.php";
            }
        }
        sort($expected);
        $written = self::tree("{$out}/Generated");
        self::assertSame($expected, $written);
        foreach ($written as $file) {
            self::assertSame((string) file_get_contents(self::COMMITTED . "/{$file}"), (string) file_get_contents("{$out}/Generated/{$file}"), $file);
        }
        $first = array_map(static fn(string $f): string => (string) file_get_contents("{$out}/Generated/{$f}"), $written);
        self::generate($out);
        self::assertSame($first, array_map(static fn(string $f): string => (string) file_get_contents("{$out}/Generated/{$f}"), $written));

        // The committed classes: arms, tag values and the slice kinds.
        foreach ([CardElement::class => CardElement::ARMS, ListItem::class => ListItem::ARMS, ContextSlice::class => ContextSlice::ARMS] as $union => $arms) {
            $short = substr($union, strrpos($union, '\\') + 1);
            self::assertSame(self::UNIONS[$short], array_map(static fn(string $c): string => substr($c, strrpos($c, '\\') + 1), $arms));
            foreach ($arms as $arm) {
                self::assertContains($union, class_implements($arm), "{$arm} implements {$union}");
            }
        }
        self::assertSame(['languages', 'plan_summary', 'review_due', 'tutor_topic'], ContextSlice::KINDS);
        self::assertSame(array_keys(self::UNIONS['CardElement']), CardElement::TYPES);
        self::assertSame(['app.render', 'app.action'], array_map(static fn(AppOperation $op): string => $op->value, AppOperation::cases()));
        self::assertSame('app.card', AppOperation::REPLY);

        // D6: a third operation fails here until the core dispatches it.
        self::assertSame(array_map(static fn(AppOperation $op): string => $op->value, AppOperation::cases()), array_keys(Core::DISPATCH));

        foreach (['CardElement.php', 'CardElementHeading.php', 'ContextSliceTutorTopic.php', 'ListItemTerm.php'] as $file) {
            $mine = (string) file_get_contents("{$out}/Generated/{$file}");
            file_put_contents("{$out}/Generated/{$file}", "<?php\n// openapi-generator's\n");
            [$status, $err] = self::generate($out);
            self::assertSame(1, $status, $file);
            self::assertStringContainsString($file, $err);
            file_put_contents("{$out}/Generated/{$file}", $mine);
        }
        array_map('unlink', glob("{$out}/Generated/*.php") ?: []);
        rmdir("{$out}/Generated");
        rmdir($out);
    }

    /** @return array{int, string} */
    private static function generate(string $out): array
    {
        $command = [PHP_BINARY, self::SCRIPT, '--view=' . self::VIEW, "--out={$out}"];
        $process = proc_open($command, [1 => ['pipe', 'w'], 2 => ['pipe', 'w']], $pipes);
        self::assertIsResource($process);
        stream_get_contents($pipes[1]);
        $err = (string) stream_get_contents($pipes[2]);
        return [proc_close($process), $err];
    }

    /** @return list<string> */
    private static function tree(string $dir): array
    {
        $files = array_map('basename', glob("{$dir}/*.php") ?: []);
        sort($files);
        return $files;
    }
}
