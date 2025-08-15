# Design Document

## Overview

Serdify es una biblioteca de Rust que proporciona deserialización JSON con manejo de errores mejorado siguiendo el estándar RFC 7807. La biblioteca se construye sobre serde_json pero intercepta y transforma los errores en estructuras RFC 7807 detalladas, recolectando múltiples errores en una sola operación para evitar round-trips innecesarios.

## Architecture

### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Public API    │───▶│  Error Collector │───▶│  RFC 7807 Error │
│   (from_str)    │    │                 │    │   Structure     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Custom Visitor  │    │ JSON Pointer    │    │ InvalidParam    │
│   Pattern       │    │   Generator     │    │   Details       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Design Patterns

1. **Custom Visitor Pattern**: Implementar un visitor personalizado que continúe la deserialización incluso después de encontrar errores
2. **Error Accumulation**: Recolectar errores en lugar de fallar inmediatamente
3. **Path Tracking**: Mantener seguimiento de la ruta JSON actual durante la deserialización
4. **Type Introspection**: Usar información de tipos de Rust para generar mensajes de error detallados

## Components and Interfaces

### 1. Public API Module

```rust
pub mod api {
    pub fn from_str<T>(s: &str) -> Result<T>
    where T: serde::de::DeserializeOwned;
}
```

**Responsabilidades:**

- Proporcionar interfaz compatible con serde_json
- Inicializar el proceso de deserialización con error collection
- Manejar casos edge como JSON completamente malformado

### 2. Custom Result Type

```rust
pub enum Result<T> {
    Ok(T),
    Err(Error),
}

impl<T> Result<T> {
    pub fn unwrap(self) -> T;
    pub fn unwrap_err(self) -> Error;
    pub fn is_ok(&self) -> bool;
    pub fn is_err(&self) -> bool;
    // ... otros métodos estándar de Result
}
```

**Responsabilidades:**

- Implementar todos los métodos estándar de Result<T, E>
- Proporcionar compatibilidad con patrones de manejo de errores de Rust
- Mantener ergonomía similar a std::result::Result

### 3. Error Collector

```rust
pub struct ErrorCollector {
    errors: Vec<InvalidParam>,
    current_path: Vec<String>,
}

impl ErrorCollector {
    pub fn new() -> Self;
    pub fn push_path(&mut self, segment: &str);
    pub fn pop_path(&mut self);
    pub fn add_error(&mut self, error_info: ErrorInfo);
    pub fn into_rfc7807_error(self) -> Error;
}
```

**Responsabilidades:**

- Mantener lista de errores encontrados durante deserialización
- Rastrear la ruta JSON actual usando stack de segmentos
- Generar JSON pointers en formato "#/a/b/c"
- Convertir errores acumulados en estructura RFC 7807

### 4. Custom Deserializer

```rust
pub struct CollectingDeserializer<'de> {
    input: &'de str,
    collector: ErrorCollector,
}

impl<'de> serde::Deserializer<'de> for CollectingDeserializer<'de> {
    // Implementar todos los métodos de Deserializer
    // Continuar deserialización incluso después de errores
}
```

**Responsabilidades:**

- Implementar trait serde::Deserializer con error collection
- Continuar procesamiento después de encontrar errores
- Coordinar con ErrorCollector para rastrear rutas
- Generar información detallada de tipos esperados vs actuales

### 5. Type Information Extractor

```rust
pub struct TypeInfo {
    pub rust_type: String,    // "u64", "String", "Vec<i32>"
    pub json_format: String,  // "integer", "string", "array"
}

pub fn extract_type_info<T>() -> TypeInfo
where T: serde::de::DeserializeOwned;
```

**Responsabilidades:**

- Extraer información de tipos de Rust en tiempo de compilación
- Mapear tipos de Rust a formatos JSON equivalentes
- Proporcionar descripciones legibles para mensajes de error

## Data Models

### RFC 7807 Error Structure

El diseño mantendrá los structs exactos que ya están definidos en src/lib.rs:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Error {
    pub r#type: Option<String>, // The URI of the error. This will be implemented in future versions
    pub title: String, // A short description of the problem. This might always be: Your request parameters didn't validate.
    pub detail: Option<String>, // A more detailed description of the problem. This will be implemented in future versions.
    pub instance: Option<String>, // Where the error happened.
    pub invalid_params: Vec<InvalidParam>, // The Array of invalid parameters that didn't validate
    pub status: Option<u16>, // The HTTP status code. This will mostlikely be 400. TODO: Add option to define custom status code.
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InvalidParam {
    pub name: String,           // The name of the parameter that has failed validation.
    pub reason: Option<String>, // The reason why it has failed validation.
    pub expected: ExpectedOrActual, // The expected type of the parameter.
    pub actual: ExpectedOrActual, // The actual type of the parameter.
    pub pointer: String,        // A JSON pointer for the parameter that has failed validation.
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExpectedOrActual {
    r#type: String, // The type of the parameter
    format: String, // The format of the parameter
}

pub enum Result<T> {
    Ok(T),
    Err(Error),
}
```

**Nota**: Estos structs ya están implementados en el código actual y el diseño se basará en mantener esta estructura exacta.

## Error Handling

### Error Categories

1. **JSON Syntax Errors**: JSON malformado, llaves no balanceadas, etc.

   - Se reportan en el campo `detail` del Error principal
   - No generan InvalidParam entries

2. **Type Mismatch Errors**: Tipo esperado vs tipo actual

   - Generan InvalidParam con información detallada de tipos
   - Incluyen JSON pointer preciso

3. **Missing Field Errors**: Campos requeridos ausentes

   - Generan InvalidParam con reason "missing required field"
   - JSON pointer apunta al objeto padre

4. **Range/Constraint Errors**: Valores fuera de rango (ej: u8 con valor 300)
   - Generan InvalidParam con información de rango esperado
   - Incluyen valor actual que causó el problema

### Error Collection Strategy

```rust
// Pseudocódigo del flujo de error collection
fn deserialize_struct() -> Result<T> {
    let mut collector = ErrorCollector::new();
    let mut partial_result = PartialStruct::new();

    for field in struct_fields {
        collector.push_path(&field.name);

        match deserialize_field(field) {
            Ok(value) => partial_result.set_field(field, value),
            Err(error) => collector.add_error(error),
        }

        collector.pop_path();
    }

    if collector.has_errors() {
        Result::Err(collector.into_rfc7807_error())
    } else {
        Result::Ok(partial_result.into_complete())
    }
}
```

## Testing Strategy

### Unit Tests

1. **Type Information Tests**

   - Verificar extracción correcta de información de tipos
   - Mapeo correcto de tipos Rust a formatos JSON

2. **JSON Pointer Generation Tests**

   - Rutas simples: "#/name"
   - Rutas anidadas: "#/user/address/street"
   - Arrays: "#/users/0/name"
   - Casos edge: campos con caracteres especiales

3. **Error Collection Tests**

   - Múltiples errores en el mismo nivel
   - Errores en estructuras anidadas
   - Combinación de errores de tipo y campos faltantes

4. **Result Implementation Tests**
   - Todos los métodos estándar (.unwrap(), .is_ok(), etc.)
   - Comportamiento consistente con std::result::Result

### Integration Tests

1. **Compatibility Tests**

   - Comparar comportamiento con serde_json para casos exitosos
   - Verificar que tipos válidos se deserialicen correctamente

2. **Complex Structure Tests**

   - Estructuras anidadas profundas
   - Arrays de objetos complejos
   - Enums y tipos opcionales

3. **Error Scenario Tests**
   - JSON completamente malformado
   - Múltiples errores de tipo en diferentes niveles
   - Campos faltantes en estructuras anidadas

### Performance Tests

1. **Benchmarks vs serde_json**

   - Casos exitosos (overhead mínimo esperado)
   - Casos con errores (overhead aceptable para mejor UX)

2. **Memory Usage Tests**
   - Verificar que error collection no cause memory leaks
   - Uso eficiente de memoria para estructuras grandes con errores
