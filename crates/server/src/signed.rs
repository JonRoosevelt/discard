use iroh::{PublicKey, SecretKey, Signature};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use shared::{Id, Message};

use crate::meta::Op;
use crate::voice::Envelope;

const DOMAIN: &[u8] = b"discard-signed-frame-v1\0";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Payload {
    Msg { message: Message },
    Env { envelope: Envelope },
    Op { op: Op },
    Hello { ticket: String },
    SoundSet { edit: Signed },
}

// The payload travels as a JSON string so the signature covers the exact bytes the author
// produced, not a re-serialization that could differ between versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signed {
    pub id: String,
    pub body: String,
    pub signer: String,
    pub sig: String,
}

fn signing_input(id: &str, body: &str) -> Vec<u8> {
    [DOMAIN, id.as_bytes(), b"\0", body.as_bytes()].concat()
}

impl Signed {
    pub fn new<T: Serialize>(key: &SecretKey, id: String, payload: &T) -> Option<Self> {
        let body = serde_json::to_string(payload).ok()?;
        let sig = key.sign(&signing_input(&id, &body));
        Some(Self {
            id,
            body,
            signer: key.public().to_string(),
            sig: hex::encode(sig.to_bytes()),
        })
    }

    pub fn open<T: DeserializeOwned>(&self) -> Option<(Id, T)> {
        let signer: PublicKey = self.signer.parse().ok()?;
        let sig = Signature::try_from(hex::decode(&self.sig).ok()?.as_slice()).ok()?;
        signer.verify(&signing_input(&self.id, &self.body), &sig).ok()?;
        Some((signer.to_string(), serde_json::from_str(&self.body).ok()?))
    }
}

pub fn authorize(envelope: Envelope, signer: &str) -> Option<Envelope> {
    let author = match &envelope {
        Envelope::PresenceJoin { member, .. } | Envelope::PresenceUpdate { member, .. } => {
            &member.user.id
        }
        Envelope::PresenceLeave { user_id, .. } => user_id,
        Envelope::PresenceSync { from, .. } | Envelope::Signal { from, .. } => from,
        Envelope::Sound { by, .. } => by,
        // Only the signer can vouch for its own presence; entries about others are dropped.
        Envelope::PresenceState {
            channel_id,
            members,
        } => {
            return Some(Envelope::PresenceState {
                channel_id: channel_id.clone(),
                members: members
                    .iter()
                    .filter(|m| m.user.id == signer)
                    .cloned()
                    .collect(),
            })
        }
    };
    (author == signer).then_some(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::SignalData;

    fn signal(from: &str) -> Payload {
        Payload::Env {
            envelope: Envelope::Signal {
                channel_id: "c".into(),
                to: "x".into(),
                from: from.into(),
                data: SignalData::Hello,
                session: None,
            },
        }
    }

    #[test]
    fn signatures() {
        let alice = SecretKey::generate();
        let bob = SecretKey::generate();
        let alice_id = alice.public().to_string();

        let good = Signed::new(&alice, "1".into(), &signal(&alice_id)).unwrap();
        let (signer, payload) = good.open::<Payload>().expect("valid signature");
        assert_eq!(signer, alice_id);
        let Payload::Env { envelope } = payload else { panic!() };
        assert!(authorize(envelope, &signer).is_some());

        let mut tampered = good.clone();
        tampered.body = tampered.body.replace("hello", "offer");
        assert!(tampered.open::<Payload>().is_none());
        let mut reid = good.clone();
        reid.id = "2".into();
        assert!(reid.open::<Payload>().is_none());

        let mut claimed = good.clone();
        claimed.signer = bob.public().to_string();
        assert!(claimed.open::<Payload>().is_none());

        let spoof = Signed::new(&bob, "3".into(), &signal(&alice_id)).unwrap();
        let (signer, payload) = spoof.open::<Payload>().expect("bob's own signature is valid");
        let Payload::Env { envelope } = payload else { panic!() };
        assert!(authorize(envelope, &signer).is_none());
    }
}
