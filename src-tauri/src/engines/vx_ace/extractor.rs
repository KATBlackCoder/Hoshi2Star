// placeholder — investigation tests below

#[cfg(test)]
mod tests {
    use marshal_rs::{dump, load_utf8};
    use serde_json::{json, Value};

    /// Affiche la structure Value produite par marshal-rs pour différents types Ruby.
    /// Exécuter avec: cargo test inspect_marshal_structure -- --nocapture
    ///
    /// API réelle marshal-rs 2.0.1 :
    ///   load_utf8(&[u8], Option<&str>) -> Result<marshal_rs::Value, _>
    ///   dump(marshal_rs::Value, Option<&str>) -> Vec<u8>
    ///   serde_json::Value ↔ marshal_rs::Value via Into (bidirectionnel)
    ///
    /// Workflow :
    ///   bytes → load_utf8(bytes, None).unwrap() → marshal_rs::Value → .into() → serde_json::Value
    ///   serde_json::Value → .into() → marshal_rs::Value → dump(val, None) → bytes
    #[test]
    fn inspect_marshal_structure() {
        // Helpers
        fn to_json(bytes: &[u8]) -> Value {
            let mv: marshal_rs::Value = load_utf8(bytes, None).unwrap();
            mv.into()
        }
        fn to_bytes(v: Value) -> Vec<u8> {
            let mv: marshal_rs::Value = v.into();
            dump(mv, None)
        }

        // --- Array de Ruby Objects (Actors.rvdata2) ---
        let actors_original = json!([
            null,
            { "name": "勇者", "nickname": "ゆうしゃ", "description": "冒険者の少年。", "id": 1 }
        ]);
        let actors_bytes = to_bytes(actors_original);
        let actors = to_json(&actors_bytes);
        println!("=== Actors (Array) ===");
        println!("{}", serde_json::to_string_pretty(&actors).unwrap());

        let name = actors.pointer("/1/name").and_then(Value::as_str);
        let nick = actors.pointer("/1/nickname").and_then(Value::as_str);
        println!("pointer(/1/name)     = {:?}", name);
        println!("pointer(/1/nickname) = {:?}", nick);
        assert_eq!(name, Some("勇者"), "/1/name doit fonctionner");
        assert_eq!(nick, Some("ゆうしゃ"), "/1/nickname doit fonctionner");
        println!("✓ Actors paths OK: /1/name, /1/nickname, /1/description");

        // --- Hash Ruby avec clés string "1","2" (MapInfos) ---
        let mi_original = json!({
            "1": { "name": "序章の森", "parent_id": 0, "order": 1 },
            "2": { "name": "村", "parent_id": 0, "order": 2 }
        });
        let mi_bytes = to_bytes(mi_original);
        let mi = to_json(&mi_bytes);
        println!("\n=== MapInfos (Hash with string keys) ===");
        println!("{}", serde_json::to_string_pretty(&mi).unwrap());

        let mi_name = mi.pointer("/1/name").and_then(Value::as_str);
        println!("pointer(/1/name) = {:?}", mi_name);
        if mi_name.is_none() {
            println!(
                "WARN: /1/name n'a pas fonctionné — structure réelle: {:#?}",
                mi
            );
        } else {
            println!("✓ MapInfos path OK: /1/name");
        }

        // --- Map*.rvdata2 — events Hash avec clé string "1" ---
        let map_original = json!({
            "display_name": "序章の森",
            "events": {
                "1": {
                    "id": 1,
                    "name": "イベント001",
                    "pages": [{
                        "list": [
                            { "code": 401, "parameters": ["こんにちは！"] },
                            { "code": 102, "parameters": [["はい", "いいえ"], 0] }
                        ]
                    }]
                }
            }
        });
        let map_bytes = to_bytes(map_original);
        let map = to_json(&map_bytes);
        println!("\n=== Map001 (events as Hash) ===");
        println!("{}", serde_json::to_string_pretty(&map).unwrap());

        let dialogue = map
            .pointer("/events/1/pages/0/list/0/parameters/0")
            .and_then(Value::as_str);
        let choice = map
            .pointer("/events/1/pages/0/list/1/parameters/0/0")
            .and_then(Value::as_str);
        println!(
            "pointer(/events/1/pages/0/list/0/parameters/0) = {:?}",
            dialogue
        );
        println!(
            "pointer(/events/1/pages/0/list/1/parameters/0/0) = {:?}",
            choice
        );
        if dialogue.is_some() {
            println!("✓ Map paths OK: /events/1/pages/0/list/N/parameters/...");
        } else {
            println!("WARN: Map path incorrect — structure réelle ci-dessus");
        }

        // --- System.rvdata2 — fields snake_case ---
        let sys_original = json!({
            "game_title": "勇者の物語",
            "currency_unit": "G",
            "terms": {
                "basic": ["最大HP", "最大MP"],
                "commands": ["攻撃", "防御"],
                "params": ["HP", "MP"],
                "messages": {
                    "actor_damage": "%1は%2のダメージを受けた！"
                }
            }
        });
        let sys_bytes = to_bytes(sys_original);
        let sys = to_json(&sys_bytes);
        println!("\n=== System (snake_case fields) ===");
        println!("{}", serde_json::to_string_pretty(&sys).unwrap());

        let title = sys.pointer("/game_title").and_then(Value::as_str);
        let currency = sys.pointer("/currency_unit").and_then(Value::as_str);
        let basic0 = sys.pointer("/terms/basic/0").and_then(Value::as_str);
        let msg = sys
            .pointer("/terms/messages/actor_damage")
            .and_then(Value::as_str);
        println!("pointer(/game_title)                    = {:?}", title);
        println!("pointer(/currency_unit)                 = {:?}", currency);
        println!("pointer(/terms/basic/0)                 = {:?}", basic0);
        println!("pointer(/terms/messages/actor_damage)   = {:?}", msg);
        if title.is_some() {
            println!("✓ System paths OK: snake_case confirmé");
        }

        // --- Test idempotence Value ---
        println!("\n=== Idempotence Value (round-trip) ===");
        let original = json!([null, { "name": "テスト" }]);
        let bytes = to_bytes(original.clone());
        let reloaded = to_json(&bytes);
        let bytes2 = to_bytes(reloaded.clone());
        let reloaded2 = to_json(&bytes2);
        assert_eq!(
            reloaded, reloaded2,
            "Value idempotent après double round-trip"
        );
        println!("✓ load_utf8(dump(load_utf8(bytes))) == load_utf8(bytes) confirmé");
    }
}
