use fstrings::f;

pub fn join_stream(stream: &str, relay: &str, length: &str, result: &str) -> String {
    f!(r#"
        (new $monotonic_stream
            (seq
                (fold ${stream} elem
                    (seq
                        (ap elem $monotonic_stream)
                        (seq
                            (canon relay $monotonic_stream #result)
                            (xor
                                (match #result.length {length} 
                                    (null) ;; fold ends if there's no `next`
                                )
                                (next elem)
                            )
                        )
                    )
                )
                (canon {relay} ${stream} #{result})
            )
        )
    "#)
}
