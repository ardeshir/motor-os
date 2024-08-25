use std::sync::Arc;

use moto_ipc::sync::{LocalServerConnection, RequestHeader};
use moto_sys_io::stats::*;

pub fn spawn_stats_service() {
    let _ = std::thread::spawn(|| stats_service_thread());
}

fn stats_service_thread() -> ! {
    let mut service = match moto_ipc::sync::LocalServer::new(
        URL_IO_STATS,
        moto_ipc::sync::ChannelSize::Small,
        2,
        2,
    ) {
        Ok(s) => s,
        Err(err) => {
            crate::moto_log!(
                "{}:{} error starting IO stats service: {:?}.",
                file!(),
                line!(),
                err
            );
            std::process::exit(-1)
        }
    };

    loop {
        match service.wait(moto_sys::SysHandle::NONE, &[]) {
            Ok(wakers) => {
                for waker in &wakers {
                    process_ipc(&mut service, *waker);
                }
            }
            Err(wakers) => assert_eq!(wakers.len(), 0),
        }
    }
}

fn process_ipc(service: &mut moto_ipc::sync::LocalServer, waker: moto_sys::SysHandle) {
    let conn = if let Some(conn) = service.get_connection(waker) {
        conn
    } else {
        // A spurious wakeup by a dropped connection.
        return;
    };
    assert!(conn.connected());
    if !conn.have_req() {
        return;
    }

    let cmd = conn.req::<RequestHeader>().cmd;
    match cmd {
        CMD_TCP_STATS => get_tcp_stats(conn),
        _ => {
            conn.disconnect();
        }
    }
}

pub struct GetTcpStatsPayload {
    pub start_id: u64,
    pub results: moto_runtime::util::SpinLock<Vec<TcpSocketStatsV1>>,
}

fn get_tcp_stats(conn: &mut LocalServerConnection) {
    let req = conn.req::<GetTcpSocketStatsRequest>();
    let start_id = req.start_id;

    let payload = Arc::new(GetTcpStatsPayload {
        start_id,
        results: moto_runtime::util::SpinLock::new(Vec::new()),
    });

    super::internal_queue::call(CMD_TCP_STATS, payload.clone());

    let resp =
        conn.resp::<GetTcpSocketStatsResponse<{ moto_sys_io::stats::MAX_TCP_SOCKET_STATS }>>();

    let mut results = vec![];
    core::mem::swap(&mut *payload.results.lock(line!()), &mut results);
    assert!(results.len() <= moto_sys_io::stats::MAX_TCP_SOCKET_STATS);
    resp.num_results = results.len() as u64;

    for idx in 0..results.len() {
        resp.socket_stats[idx] = results[idx];
    }

    resp.header.result = moto_sys::ErrorCode::Ok.into();
    let _ = conn.finish_rpc();
}
