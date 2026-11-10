//  fast-uuid-v7
//  © Copyright 2026, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/fast-uuid-v7

use napi::bindgen_prelude::*;
use napi::sys;
use napi_derive::napi;

const UUID_LEN: usize = 16;

/// ASCII string that is handed to V8 as a one-byte (latin1) string straight
/// from the stack, skipping the UTF-8 validation of `napi_create_string_utf8`.
pub struct AsciiStr<const N: usize>([u8; N]);

impl<const N: usize> TypeName for AsciiStr<N> {
    fn type_name() -> &'static str {
        "String"
    }

    fn value_type() -> ValueType {
        ValueType::String
    }
}

impl<const N: usize> ToNapiValue for AsciiStr<N> {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> Result<sys::napi_value> {
        let mut raw = std::ptr::null_mut();
        // SAFETY: `val.0` is a valid buffer of N bytes for the duration of the call.
        let status = unsafe {
            sys::napi_create_string_latin1(env, val.0.as_ptr().cast(), N as isize, &mut raw)
        };
        if status != sys::Status::napi_ok {
            return Err(Error::new(Status::from(status), "Failed to create string"));
        }
        Ok(raw)
    }
}

fn uuid_str(id: u128) -> AsciiStr<36> {
    AsciiStr(*fast_uuid_v7::format_uuid(id).as_bytes())
}

fn uuid_hex(id: u128) -> AsciiStr<32> {
    AsciiStr(*fast_uuid_v7::format_uuid_hex(id).as_bytes())
}

fn fill(target: &mut [u8], mut next: impl FnMut() -> u128) -> u32 {
    let (chunks, _) = target.as_chunks_mut::<UUID_LEN>();
    for chunk in chunks.iter_mut() {
        *chunk = next().to_be_bytes();
    }
    chunks.len() as u32
}

fn id_from_bigint(value: BigInt) -> Result<u128> {
    let (negative, id, lossless) = value.get_u128();
    if negative || !lossless {
        return Err(Error::new(
            Status::InvalidArg,
            "UUID must be an unsigned 128-bit BigInt",
        ));
    }
    Ok(id)
}

fn id_from_bytes(bytes: &[u8]) -> Result<u128> {
    let bytes: [u8; UUID_LEN] = bytes.try_into().map_err(|_| {
        Error::new(
            Status::InvalidArg,
            format!("UUID buffer must be {UUID_LEN} bytes, got {}", bytes.len()),
        )
    })?;
    Ok(u128::from_be_bytes(bytes))
}

/// UUID v7 as BigInt, 74 random bits.
#[napi]
pub fn gen_id() -> u128 {
    fast_uuid_v7::gen_id()
}

/// UUID v7 as canonical string `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
#[napi(ts_return_type = "string")]
pub fn gen_id_str() -> AsciiStr<36> {
    uuid_str(fast_uuid_v7::gen_id())
}

/// UUID v7 as 32 lowercase hex chars without dashes.
#[napi(ts_return_type = "string")]
pub fn gen_id_hex() -> AsciiStr<32> {
    uuid_hex(fast_uuid_v7::gen_id())
}

/// Fills `target` with as many 16-byte big-endian UUIDs as fit and returns
/// that count. Trailing bytes that do not fit a whole UUID are left unchanged.
#[napi]
pub fn fill_ids(mut target: BufferSlice) -> u32 {
    fill(&mut target, fast_uuid_v7::gen_id)
}

/// Per-thread monotonic UUID v7 as BigInt (56 random bits + 18-bit counter).
#[napi]
pub fn gen_id_with_count() -> u128 {
    fast_uuid_v7::gen_id_with_count()
}

/// Per-thread monotonic UUID v7 as canonical string.
#[napi(ts_return_type = "string")]
pub fn gen_id_with_count_str() -> AsciiStr<36> {
    uuid_str(fast_uuid_v7::gen_id_with_count())
}

/// UUID v7 as BigInt with a 4-bit sub-millisecond fraction in `rand_a`.
#[napi]
pub fn gen_id_with_sub_ms_4() -> u128 {
    fast_uuid_v7::gen_id_with_sub_ms_4()
}

/// UUID v7 as BigInt with an 8-bit sub-millisecond fraction in `rand_a`.
#[napi]
pub fn gen_id_with_sub_ms_8() -> u128 {
    fast_uuid_v7::gen_id_with_sub_ms_8()
}

/// UUID v7 as BigInt with a 12-bit sub-millisecond fraction in `rand_a`.
#[napi]
pub fn gen_id_with_sub_ms_12() -> u128 {
    fast_uuid_v7::gen_id_with_sub_ms_12()
}

/// Formats a BigInt or 16-byte Buffer as canonical UUID string.
#[napi(ts_return_type = "string")]
pub fn format_uuid(id: Either<BigInt, BufferSlice>) -> Result<AsciiStr<36>> {
    let id = match id {
        Either::A(value) => id_from_bigint(value)?,
        Either::B(bytes) => id_from_bytes(&bytes)?,
    };
    Ok(uuid_str(id))
}

// `SequentialGenerator` is exposed as an `External` handle plus free functions
// and wrapped in a JS class: this is ~2.5x faster per call than a `#[napi]`
// class, whose methods pay for unwrapping `this` on every call.
type Sequential = External<fast_uuid_v7::SequentialGenerator>;

/// Creates the native state behind a JS `SequentialGenerator`.
#[napi]
pub fn sequential_create() -> Sequential {
    External::new(fast_uuid_v7::SequentialGenerator::new())
}

/// Next strictly increasing id of `generator` as BigInt.
#[napi]
pub fn sequential_next_id(generator: &mut Sequential) -> u128 {
    generator.next_id()
}

/// Next strictly increasing id of `generator` as canonical string.
#[napi(ts_return_type = "string")]
pub fn sequential_next_id_str(generator: &mut Sequential) -> AsciiStr<36> {
    uuid_str(generator.next_id())
}

/// Fills `target` with consecutive ids of `generator`, see [`fill_ids`].
#[napi]
pub fn sequential_fill_ids(generator: &mut Sequential, mut target: BufferSlice) -> u32 {
    fill(&mut target, || generator.next_id())
}
