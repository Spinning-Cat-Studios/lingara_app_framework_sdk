<?php

declare(strict_types=1);

// The PHP fixture app's launcher (ADR 30.9.26am D9). PHP has a built-in
// server but no way to ask it for a free port and hear back which one, so
// this finds a free loopback port, starts `php -S 127.0.0.1:<port>` over
// fixture.php, waits until it accepts a connection, and only then prints
// `listening <port>` as its first line on standard output.
//
//   php serve.php <nyholm|guzzle>
//
// The server runs under `sh`, which kills it when this process's stdin pipe
// to it closes: the host stops the launcher with SIGKILL, and an orphaned
// server would otherwise keep running. The server's own output goes to
// standard error, so standard output carries the handshake alone.

function fail(string $message): never
{
    // Standard error: standard output's first line is the handshake.
    fwrite(STDERR, "serve.php: {$message}\n");
    exit(1);
}

$variant = $argv[1] ?? 'nyholm';
in_array($variant, ['nyholm', 'guzzle'], true) || fail('the variant is nyholm or guzzle');
$wanted = (int) (getenv('LINGARA_APPS_CONFORMANCE_PORT') ?: 0);

function freePort(): int
{
    $socket = stream_socket_server('tcp://127.0.0.1:0', $errno, $error) ?: fail($error ?? 'no free port');
    $name = (string) stream_socket_get_name($socket, false);
    fclose($socket);
    return (int) substr($name, strrpos($name, ':') + 1);
}

/** @param resource $process */
function ready(int $port, mixed $process): bool
{
    $deadline = microtime(true) + 20;
    while (microtime(true) < $deadline && proc_get_status($process)['running']) {
        $probe = @fsockopen('127.0.0.1', $port, $errno, $error, 0.2);
        if ($probe !== false) {
            fclose($probe);
            return true;
        }
        usleep(20_000);
    }
    return false;
}

/** @return array{resource, array<int, resource>} */
function start(int $port, string $variant): array
{
    $server = escapeshellarg(PHP_BINARY) . " -d expose_php=0 -S 127.0.0.1:{$port} " . escapeshellarg(__DIR__ . '/fixture.php');
    $command = ['sh', '-c', "{$server} <&- & s=\$!; cat > /dev/null; kill \$s"];
    $env = getenv();
    $env['LINGARA_APPS_PSR7'] = $variant;
    $process = proc_open($command, [0 => ['pipe', 'r'], 1 => STDERR, 2 => STDERR], $pipes, null, $env);
    $process !== false || fail('could not start php -S');
    return [$process, $pipes];
}

// A port freed above can be taken before php -S binds it: try a few.
for ($attempt = 0; $attempt < 5; $attempt++) {
    $port = $wanted !== 0 ? $wanted : freePort();
    [$process, $pipes] = start($port, $variant);
    if (ready($port, $process)) {
        fwrite(STDOUT, "listening {$port}\n");
        fflush(STDOUT);
        // Not proc_close(): it closes the stdin pipe first, which stops the server.
        while (proc_get_status($process)['running']) {
            sleep(1);
        }
        exit(0);
    }
    fclose($pipes[0]);
    proc_close($process);
}
fail('php -S did not start');
