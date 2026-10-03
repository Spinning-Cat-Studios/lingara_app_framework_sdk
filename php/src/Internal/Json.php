<?php

declare(strict_types=1);

namespace Lingara\Apps\Internal;

use Lingara\Apps\Generated\ModelInterface;
use Lingara\Apps\Limits;

/**
 * The generic JSON tree the rules read: a decoded `stdClass` tree, PHP
 * arrays, or the kit's own models, read alike.
 *
 * @internal
 */
final class Json
{
    private function __construct() {}

    /**
     * The value as plain JSON data: a generated model or arm becomes its
     * wire form (an unset optional member omitted), an enum its value.
     * Objects stay objects (`stdClass` or string-keyed arrays) and lists
     * stay lists.
     */
    public static function plain(mixed $value): mixed
    {
        return match (true) {
            $value instanceof ModelInterface => self::model($value),
            $value instanceof \JsonSerializable => self::plain($value->jsonSerialize()),
            $value instanceof \BackedEnum => $value->value,
            $value instanceof \stdClass => (object) array_map(self::plain(...), get_object_vars($value)),
            is_array($value) => array_map(self::plain(...), $value),
            default => $value,
        };
    }

    /** One member of an object, or null when absent or not an object. */
    public static function member(mixed $object, string $key): mixed
    {
        if ($object instanceof \stdClass) {
            return property_exists($object, $key) ? $object->{$key} : null;
        }
        return is_array($object) && !array_is_list($object) ? ($object[$key] ?? null) : null;
    }

    /** Whether an object has a member, null or not. */
    public static function has(mixed $object, string $key): bool
    {
        if ($object instanceof \stdClass) {
            return property_exists($object, $key);
        }
        return is_array($object) && array_key_exists($key, $object);
    }

    /** Whether the value is a JSON object: a `stdClass`, or a non-empty string-keyed array. */
    public static function isObject(mixed $value): bool
    {
        return $value instanceof \stdClass || (is_array($value) && $value !== [] && !array_is_list($value));
    }

    /**
     * A JSON array's elements, or an empty list for anything else.
     *
     * @return list<mixed>
     */
    public static function items(mixed $value): array
    {
        return is_array($value) && array_is_list($value) ? $value : [];
    }

    /**
     * An object's members.
     *
     * @return array<mixed>
     */
    public static function members(mixed $object): array
    {
        if ($object instanceof \stdClass) {
            return get_object_vars($object);
        }
        return is_array($object) ? $object : [];
    }

    /** The value as the bytes the kit sends, or null when it cannot be encoded. */
    public static function encode(mixed $value): ?string
    {
        try {
            return json_encode($value, Limits::JSON_FLAGS);
        } catch (\JsonException) {
            return null;
        }
    }

    /** @return array<string, mixed> */
    private static function model(ModelInterface $model): array
    {
        $wire = [];
        foreach ($model::attributeMap() as $property => $name) {
            // Read through ArrayAccess, which every generated model has: a
            // getter's return type throws on a member never set.
            $value = $model instanceof \ArrayAccess ? $model[$property] : null;
            if ($value !== null && is_string($name)) {
                $wire[$name] = self::plain($value);
            }
        }
        return $wire;
    }
}
