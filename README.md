# Serdify ![Crates.io Version](https://img.shields.io/crates/v/serdify?link=https%3A%2F%2Fcrates.io%2Fcrates%2Fserdify) ![docs.rs](https://img.shields.io/docsrs/serdify?logo=rust&link=https%3A%2F%2Fdocs.rs%2Fserdify) ![GitHub branch status](https://img.shields.io/github/checks-status/TheCukitoDev/serdify/main) [![Socket Badge](https://socket.dev/api/badge/cargo/package/serdify/0.1.0)](https://socket.dev/cargo/package/serdify/overview/0.1.0)

## What is Serdify?

Serdify is a Rust library designed to simplify error handling when working with JSON objects. It provides a set of utilities to help developers manage and transform errors that may occur during the serialization and deserialization process.

## Our standard procedure

We receive a JSON object and attempt to deserialize it into a Rust struct. If the deserialization fails, we use Serdify to transform the error into a more user-friendly format following the [RFC 7807 specification](https://datatracker.ietf.org/doc/html/rfc7807).
