//! Token transfer validation as a native validity predicate

use std::collections::HashSet;
use std::str::FromStr;

use borsh::BorshDeserialize;
#[cfg(not(feature = "ABCI"))]
use ibc::applications::ics20_fungible_token_transfer::msgs::transfer::MsgTransfer;
#[cfg(not(feature = "ABCI"))]
use ibc::core::ics04_channel::msgs::PacketMsg;
#[cfg(not(feature = "ABCI"))]
use ibc::core::ics04_channel::packet::Packet;
#[cfg(not(feature = "ABCI"))]
use ibc::core::ics26_routing::msgs::Ics26Envelope;
#[cfg(feature = "ABCI")]
use ibc_abci::applications::ics20_fungible_token_transfer::msgs::transfer::MsgTransfer;
#[cfg(feature = "ABCI")]
use ibc_abci::core::ics04_channel::msgs::PacketMsg;
#[cfg(feature = "ABCI")]
use ibc_abci::core::ics04_channel::packet::Packet;
#[cfg(feature = "ABCI")]
use ibc_abci::core::ics26_routing::msgs::Ics26Envelope;
use thiserror::Error;

use crate::ledger::native_vp::{self, Ctx, NativeVp};
use crate::ledger::storage::{self as ledger_storage, StorageHasher};
use crate::types::address::{Address, Error as AddressError, InternalAddress};
use crate::types::ibc::data::{
    Error as IbcDataError, FungibleTokenPacketData, IbcMessage,
};
use crate::types::storage::Key;
use crate::types::token::{self, Amount, AmountParseError};
use crate::vm::WasmCacheAccess;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("Native VP error: {0}")]
    NativeVpError(native_vp::Error),
    #[error("IBC message error: {0}")]
    IbcMessage(IbcDataError),
    #[error("Invalid message error")]
    InvalidMessage,
    #[error("Invalid address error")]
    Address(AddressError),
    #[error("Token error")]
    NoToken,
    #[error("Token error")]
    Amount(AmountParseError),
    #[error("Decoding error")]
    Decoding(std::io::Error),
    #[error("Decoding PacketData error")]
    DecodingPacketData(serde_json::Error),
    #[error("Invalid token transfer error")]
    TokenTransfer(String),
}

/// Result for Token VP
pub type Result<T> = std::result::Result<T, Error>;

/// Token native VP for escrow, unescrow, burn, and mint
pub struct Token<'a, DB, H, CA>
where
    DB: ledger_storage::DB + for<'iter> ledger_storage::DBIter<'iter>,
    H: StorageHasher,
    CA: 'static + WasmCacheAccess,
{
    /// Context to interact with the host structures.
    pub ctx: Ctx<'a, DB, H, CA>,
}

impl<'a, DB, H, CA> NativeVp for Token<'a, DB, H, CA>
where
    DB: 'static + ledger_storage::DB + for<'iter> ledger_storage::DBIter<'iter>,
    H: 'static + StorageHasher,
    CA: 'static + WasmCacheAccess,
{
    type Error = Error;

    const ADDR: InternalAddress = InternalAddress::Burn;

    fn validate_tx(
        &self,
        tx_data: &[u8],
        keys_changed: &HashSet<Key>,
        _verifiers: &HashSet<Address>,
    ) -> Result<bool> {
        // Check the message
        let ibc_msg = IbcMessage::decode(tx_data).map_err(Error::IbcMessage)?;
        match &ibc_msg.0 {
            Ics26Envelope::Ics20Msg(msg) => self.validate_sending_token(msg),
            Ics26Envelope::Ics4PacketMsg(PacketMsg::RecvPacket(msg)) => {
                self.validate_receiving_token(&msg.packet)
            }
            Ics26Envelope::Ics4PacketMsg(PacketMsg::ToPacket(msg)) => {
                self.validate_refunding_token(&msg.packet)
            }
            Ics26Envelope::Ics4PacketMsg(PacketMsg::ToClosePacket(msg)) => {
                self.validate_refunding_token(&msg.packet)
            }
            _ => Err(Error::InvalidMessage),
        }
    }
}

impl<'a, DB, H, CA> Token<'a, DB, H, CA>
where
    DB: 'static + ledger_storage::DB + for<'iter> ledger_storage::DBIter<'iter>,
    H: 'static + StorageHasher,
    CA: 'static + WasmCacheAccess,
{
    fn validate_sending_token(&self, msg: &MsgTransfer) -> Result<bool> {
        let data = FungibleTokenPacketData::from(msg.clone());
        let token_str =
            data.denomination.split('/').last().ok_or(Error::NoToken)?;
        let token = Address::decode(token_str).map_err(Error::Address)?;
        let amount = Amount::from_str(&data.amount).map_err(Error::Amount)?;

        // check the denomination field
        let prefix = format!(
            "{}/{}/",
            msg.source_port.clone(),
            msg.source_channel.clone()
        );
        let target = if data.denomination.starts_with(&prefix) {
            // sink zone
            Address::Internal(InternalAddress::Burn)
        } else {
            // source zone
            InternalAddress::ibc_escrow_address(
                msg.source_port.to_string(),
                msg.source_channel.to_string(),
            )
        };

        let target_key = token::balance_key(&token, &target);
        let pre = match self.ctx.read_pre(&target_key)? {
            Some(v) => Amount::try_from_slice(&v).map_err(Error::Decoding)?,
            None => Amount::default(),
        };
        let post = match self.ctx.read_post(&target_key)? {
            Some(v) => Amount::try_from_slice(&v).map_err(Error::Decoding)?,
            None => Amount::default(),
        };

        let change = post.change() - pre.change();
        if change == amount.change() {
            Ok(true)
        } else {
            Err(Error::TokenTransfer(format!(
                "Sending the token is invalid: {}",
                data
            )))
        }
    }

    fn validate_receiving_token(&self, packet: &Packet) -> Result<bool> {
        let data: FungibleTokenPacketData =
            serde_json::from_slice(&packet.data)
                .map_err(Error::DecodingPacketData)?;
        let token_str =
            data.denomination.split('/').last().ok_or(Error::NoToken)?;
        let token = Address::decode(token_str).map_err(Error::Address)?;
        let amount = Amount::from_str(&data.amount).map_err(Error::Amount)?;

        let prefix = format!(
            "{}/{}/",
            packet.source_port.clone(),
            packet.source_channel.clone()
        );
        let source = if data.denomination.starts_with(&prefix) {
            // this chain is the source
            InternalAddress::ibc_escrow_address(
                packet.destination_port.to_string(),
                packet.destination_channel.to_string(),
            )
        } else {
            // the sender is the source
            Address::Internal(InternalAddress::Mint)
        };

        let source_key = token::balance_key(&token, &source);
        let pre = match self.ctx.read_pre(&source_key)? {
            Some(v) => Amount::try_from_slice(&v).map_err(Error::Decoding)?,
            None => Amount::default(),
        };
        let post = match self.ctx.read_post(&source_key)? {
            Some(v) => Amount::try_from_slice(&v).map_err(Error::Decoding)?,
            None => Amount::default(),
        };

        let change = post.change() - pre.change();
        if change == amount.change() {
            Ok(true)
        } else {
            Err(Error::TokenTransfer(format!(
                "Receivinging the token is invalid: {}",
                data
            )))
        }
    }

    fn validate_refunding_token(&self, packet: &Packet) -> Result<bool> {
        let data: FungibleTokenPacketData =
            serde_json::from_slice(&packet.data)
                .map_err(Error::DecodingPacketData)?;
        let token_str =
            data.denomination.split('/').last().ok_or(Error::NoToken)?;
        let token = Address::decode(token_str).map_err(Error::Address)?;
        let amount = Amount::from_str(&data.amount).map_err(Error::Amount)?;

        // check the denomination field
        let prefix = format!(
            "{}/{}/",
            packet.source_port.clone(),
            packet.source_channel.clone()
        );
        let source = if data.denomination.starts_with(&prefix) {
            // sink zone: mint the token for the refund
            Address::Internal(InternalAddress::Mint)
        } else {
            // source zone: unescrow the token for the refund
            InternalAddress::ibc_escrow_address(
                packet.source_port.to_string(),
                packet.source_channel.to_string(),
            )
        };

        let source_key = token::balance_key(&token, &source);
        let pre = match self.ctx.read_pre(&source_key)? {
            Some(v) => Amount::try_from_slice(&v).map_err(Error::Decoding)?,
            None => Amount::default(),
        };
        let post = match self.ctx.read_post(&source_key)? {
            Some(v) => Amount::try_from_slice(&v).map_err(Error::Decoding)?,
            None => Amount::default(),
        };

        let change = post.change() - pre.change();
        if change == amount.change() {
            Ok(true)
        } else {
            Err(Error::TokenTransfer(format!(
                "Refunding the token is invalid: {}",
                data
            )))
        }
    }
}

impl From<native_vp::Error> for Error {
    fn from(err: native_vp::Error) -> Self {
        Self::NativeVpError(err)
    }
}