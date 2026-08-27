//! Handshake acceptance + downlink emission for [`super::ServerEngine`].

use std::io;
use std::net::IpAddr;

use super::super::datagram::{self, Datagram, Kind};
use super::super::keyx;
use super::ServerEngine;
use crate::covert::carrier::CarrierServer;

pub(super) fn open<S>(
    engine: &mut ServerEngine<S>,
    session_id: u32,
    token: &[u8],
    reply_to: IpAddr,
    now: f64,
) -> io::Result<()>
where
    S: CarrierServer<ReplyTo = IpAddr>,
{
    let Some(key) = keyx::unseal(&engine.identity, token) else {
        return Ok(());
    };
    if engine
        .table
        .already_belongs_to_another_key(session_id, &key)
    {
        return Ok(());
    }
    {
        let Some(s) = engine.table.open_unless_full(session_id, now, reply_to) else {
            return Ok(());
        };
        s.key = Some(key);
        s.reply_to = Some(reply_to);
    }
    respond(engine, session_id, now)
}

pub(super) fn respond<S>(engine: &mut ServerEngine<S>, session_id: u32, now: f64) -> io::Result<()>
where
    S: CarrierServer<ReplyTo = IpAddr>,
{
    let (key, reply_to, chunks, ack) = {
        let Some(s) = engine.table.already_open(session_id, now) else {
            return Ok(());
        };
        let Some(reply_to) = s.reply_to else {
            return Ok(());
        };
        let Some(key) = s.key.clone() else {
            return Ok(());
        };
        (key, reply_to, s.send.ready(now), s.recv.ack())
    };
    let pkts: Vec<(u32, Option<Vec<u8>>)> = if chunks.is_empty() {
        vec![(0, None)]
    } else {
        chunks.into_iter().map(|(s, p)| (s, Some(p))).collect()
    };
    for (seq, payload) in pkts {
        let kind = if payload.is_some() {
            Kind::Data
        } else {
            Kind::Poll
        };
        let dg = Datagram {
            session: session_id,
            seq,
            ack,
            kind,
            payload: payload.unwrap_or_default(),
        };
        let wire = datagram::pack(&dg, &key, engine.tag);
        engine.carrier.send_response(&reply_to, &wire)?;
    }
    Ok(())
}
