    #[test]
    fn test_type_info_basic() {
        let u64_info = extract_type_info::<u64>();
        assert_eq!(u64_info.rust_type, "u64");
        assert_eq!(u64_info.json_format, "integer");
    }
}