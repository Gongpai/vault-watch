#![no_main]

use libfuzzer_sys::fuzz_target;
use vault_watch::broker::{
    decode_request_frame, decode_response_frame, encode_request_frame, encode_response_frame,
};

fuzz_target!(|data: &[u8]| {
    if let Ok(request) = decode_request_frame(data) {
        let encoded = encode_request_frame(&request).expect("decoded requests must re-encode");
        let decoded = decode_request_frame(&encoded).expect("encoded requests must decode");
        assert_eq!(decoded, request);
    }
    if let Ok(response) = decode_response_frame(data) {
        let encoded =
            encode_response_frame(&response).expect("decoded responses must re-encode");
        let decoded = decode_response_frame(&encoded).expect("encoded responses must decode");
        assert_eq!(decoded, response);
    }
});
