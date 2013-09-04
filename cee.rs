extern mod usb;
extern mod extra;
use std::str;
use std::comm;
use extra::time;

#[packed]
struct CEE_version_descriptor {
	version_major: u8,
	version_minor: u8,
	flags: u8,
	per_ns: u8,
	min_per: u8,
}

fn main () {
	let c = usb::Context::new();
	c.setDebug(0);
	match c.find_by_vid_pid(0x59e3, 0xcee1) {
		Some(dev) => {
		match dev.open() {
			Ok(handle) => {
				printfln!("CEE %?", str::from_bytes_owned(handle.ctrl_read(0xC0, 0x00, 0, 0, 64).unwrap()));
				// timer / samples per second
				let xmegaPer = 4e6/1e2;
				// stop, reset, etc
				handle.ctrl_read(0x40, 0x80, 0, 0, 0);
				//let (p, c): (comm::Port<~[~[f32]]>, comm::Chan<~[~[f32]]>) = comm::stream();
				let ho = handle.clone();
				do spawn {
					ho.write_stream(0x02, usb::libusb::LIBUSB_TRANSFER_TYPE_BULK, 32, 6, |b| {
						let t = time::precise_time_ns();
						let mut y = b.unwrap();
						// simv
						y[0] = 2u8;
						// svmi
						y[1] = 1u8;
						let av = [0.02f32, ..10];
						let bv = [3f32, ..10];
						for i in range(0, 10) {
							let ab = match y[0] {
								0 => 0,
								1 => ((av[i]/5f32)*4095.0) as u16,
								2 => (4095.0*(1.25+(av[i]*10.0/(45.0*0.07)))/2.5) as u16,
								_ => fail!("mode DNI")
							};
							let bb = match y[1] {
								0 => 0,
								1 => ((bv[i]/5f32)*4095.0) as u16,
								2 => (4095.0*(1.25+(bv[i]*10.0/(45.0*0.07)))/2.5) as u16,
								_ => fail!("mode DNI")
							};
							y[i*3+2] = (ab&0xFF) as u8;
							y[i*3+3] = (bb&0xFF) as u8;
							y[i*3+4] = ((((bb&0xF00) >> 4) | ((ab&0xF00) >> 8)) & 0xFF) as u8;
						}
						true
					});
				}
				let hi = handle.clone();
				do spawn {
					hi.read_stream(0x81, usb::libusb::LIBUSB_TRANSFER_TYPE_BULK, 64, 6, |res| {
						let t = time::precise_time_ns();
						let y = res.unwrap();
						let mode_a: u8 = y[0];
						let mode_b: u8 = y[1];
						let flags: u8 = y[2];
						if (flags == 1) { println("dropped packet") }
						let mode_seq: u8 = y[3];
						let u: ~[~[uint]] = y.slice(4,64).chunk_iter(6).map(|x| {
								// av, ai, bv, bi
								~[x[0] as uint | ((x[2] as uint & 0x0F) << 8),
								x[1] as uint | ((x[2] as uint & 0xF0) << 4),
								x[3] as uint | ((x[5] as uint & 0x0F) << 8),
								x[4] as uint | ((x[5] as uint & 0xF0) << 4)]
							}).collect();
						let s: ~[~[int]] = u.iter().map(|x| {
								x.iter().map(|&k: &uint| {
									if (k > (1 << 11) - 1)
										{ k as int - (1 << 12) }
									else { k as int }
								}).collect()
							}).collect();
						let v: ~[~[f32]] = s.iter().map(|x| {
							~[x[0] as f32 * 5.0/2048.0,
							x[1] as f32 * 2.5/2048.0/(0.07*45.0*2.0),
							x[2] as f32 * 5.0/2048.0,
							x[3] as f32 * 2.5/2048.0/(0.07*45.0*2.0)]
							}).collect();
						printfln!("%?", v[0]);
						true
					});
				}
				// start
				handle.ctrl_read(0x40, 0x80, (xmegaPer as u16), 1, 0);
			},
			Err(code) => {printfln!("Error opening device: %?", code);}
		}}
		None => println("Device not found"),
	};
}
