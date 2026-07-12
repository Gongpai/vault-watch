#![no_main]

use libfuzzer_sys::fuzz_target;
use vault_watch::broker::{decode_request_frame, encode_request_frame};

fuzz_target!(|data: &[u8]| {
    if let Ok(request) = decode_request_frame(data) {
        let encoded = encode_request_frame(&request).expect("decoded requests must re-encode");
        let decoded = decode_request_frame(&encoded).expect("encoded requests must decode");
        assert_eq!(decoded, request);
    }
});
