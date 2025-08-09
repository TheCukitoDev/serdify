# Serdify

## What is Serdify?

Serdify is a Rust library designed to simplify error handling when working with JSON objects. It provides a set of utilities to help developers manage and transform errors that may occur during the serialization and deserialization process.

## Our standard procedure

We receive a JSON object and attempt to deserialize it into a Rust struct. If the deserialization fails, we use Serdify to transform the error into a more user-friendly format following the [RFC 7807 specification](https://datatracker.ietf.org/doc/html/rfc7807).
