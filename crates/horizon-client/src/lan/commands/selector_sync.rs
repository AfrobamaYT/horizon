//! Horizon's selector-token metadata, sent beside the server command tree.
use horizon_client_core::registries::SelectorToken;
use horizon_protocol::{
    Error, Result,
    buf::{Reader, Writer},
};
pub const CHANNEL: &str = "horizon:selector_tokens";
pub fn encode(tokens: &[SelectorToken]) -> Vec<u8> {
    let mut w = Writer::new();
    w.varint(tokens.len() as i32);
    for token in tokens {
        w.string(&token.name);
        w.string(&token.tooltip);
        w.bool(token.players_only);
        w.bool(token.limit != usize::MAX);
        if token.limit != usize::MAX {
            w.u32(token.limit as u32);
        }
        w.u8(match token.sort {
            "nearest" => 1,
            "furthest" => 2,
            "random" => 3,
            _ => 0,
        });
    }
    w.data
}
pub fn decode(data: &[u8]) -> Result<Vec<SelectorToken>> {
    let mut r = Reader::new(data);
    let count = r.varint()?;
    if count < 0 || count as usize > r.remaining() {
        return Err(Error::new("selector tokens: invalid count"));
    }
    let mut tokens = Vec::new();
    for _ in 0..count {
        let name = r.string(32767)?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            || ["p", "a", "r", "s", "e", "n"].contains(&name.as_str())
            || tokens
                .iter()
                .any(|token: &SelectorToken| token.name == name)
        {
            return Err(Error::new("selector tokens: invalid or duplicate name"));
        }
        let tooltip = r.string(32767)?;
        let players_only = r.bool()?;
        let limit = if r.bool()? {
            let n = r.u32()?;
            if n == 0 {
                return Err(Error::new("selector tokens: zero limit"));
            }
            n as usize
        } else {
            usize::MAX
        };
        let sort = match r.u8()? {
            0 => "arbitrary",
            1 => "nearest",
            2 => "furthest",
            3 => "random",
            _ => return Err(Error::new("selector tokens: invalid sort")),
        };
        tokens.push(SelectorToken {
            name,
            tooltip,
            players_only,
            limit,
            sort,
        });
    }
    if r.remaining() != 0 {
        return Err(Error::new("selector tokens: trailing bytes"));
    }
    Ok(tokens)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_tokens_install_literal_metadata_and_reject_partial_snapshots() {
        let tokens = vec![SelectorToken {
            name: "allies".into(),
            tooltip: "test.allies".into(),
            players_only: true,
            limit: 1,
            sort: "nearest",
        }];
        let data = encode(&tokens);
        assert_eq!(decode(&data).unwrap(), tokens);
        for end in 0..data.len() {
            assert!(decode(&data[..end]).is_err());
        }
        let mut trailing = data;
        trailing.push(0);
        assert!(decode(&trailing).is_err());
        assert!(decode(&encode(&[tokens[0].clone(), tokens[0].clone()])).is_err());
        assert!(decode(&encode(&[])).unwrap().is_empty());
    }
}
