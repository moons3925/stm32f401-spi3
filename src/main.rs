#![no_std]
#![no_main]

use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
use cortex_m_rt::entry;
use stm32f4xx_hal::prelude::*;  // Hz()
use stm32f4xx_hal::gpio::*;

use stm32f4xx_hal::rcc::RccExt;
use stm32f4xx_hal::spi::*;
use stm32f4xx_hal::pac::*;

const WHO_AM_I: u8 = 0x0f;  // デバイス確認用のコマンド
const CTRL_REG1: u8 = 0x20; // コントロールレジスタ1
const WAKE_UP: u8 = 0x90;   // デバイスを起こすためのコマンド
const P_ADRS: u8 = 0x28;    // 気圧読み込み用のアドレス
const LPS25HB_DEVICE_CODE: u8 = 0xbd;

#[entry]
fn main() -> ! {

    let dp = stm32f4xx_hal::pac::Peripherals::take().unwrap();
    let gpioa = dp.GPIOA.split();   // GPIOAのclockも有効にしてくれる （AHBENRレジスタ）
    let mut cs = DigitalOut::new(gpioa.pa4);

    let rcc = dp.RCC.constrain();   // RCCの取得
    let clks = rcc.cfgr.freeze();   // 各clockの設定
    let mode = Mode { polarity: Polarity::IdleHigh, phase: Phase::CaptureOnSecondTransition };  // SPIのモード
    let hz = 1000_000u32.Hz();

    lps25hb_deselect(&mut cs);  // CS=Highにしておく
    let mut spi = Spi::new(
        dp.SPI1,
        (gpioa.pa5, gpioa.pa6, gpioa.pa7),
        mode,
        hz,
        &clks,
    );

    lps25hb_init(&mut spi, &mut cs);    // LPS25HBの初期化
    loop {

        let mut data: [u8; 4] = [P_ADRS | 0xc0, 0, 0, 0];

        lps25hb_select(&mut cs);
        lps25hb_send_buf(&mut spi, &mut data);
        lps25hb_deselect(&mut cs);
        let mut press = (data[3] as u32) << 16_u32 | (data[2] as u32) << 8_u32 | data[1] as u32;
        press >>= 12_i32;   // 1/4096
    }
}

fn lps25hb_init(spi: &mut Spi<SPI1, (PA5, PA6, PA7), TransferModeNormal>, cs: &mut DigitalOut) -> bool {

    lps25hb_select(cs);
    lps25hb_send(spi, WHO_AM_I | 0x80);     // WHO_AM_I コマンドを送る
    let res = lps25hb_send(spi, 0u8);   // 返事を読む
    lps25hb_deselect(cs);

    lps25hb_select(cs);
    lps25hb_send(spi, CTRL_REG1);           // CTRLREG1
    lps25hb_send(spi, WAKE_UP);             // 起床を指示
    lps25hb_deselect(cs);
    if res == LPS25HB_DEVICE_CODE { // デバイスコードが返ること
        return true;
    }
    false
}

fn lps25hb_select(cs: &mut DigitalOut) {    // CS=Low
    cs.select();
}

fn lps25hb_deselect(cs: &mut DigitalOut) {  // CS=High
    cs.deselect();
}

fn lps25hb_send(spi: &mut Spi<SPI1, (PA5, PA6, PA7), TransferModeNormal>, data: u8) -> u8 {
    while !spi.is_txe() {}
    spi.send(data).unwrap();    // 送って
    while !spi.is_rxne() {}
    spi.read().unwrap() // 読む
}

fn lps25hb_send_buf(spi: &mut Spi<SPI1, (PA5, PA6, PA7), TransferModeNormal>, data: &mut [u8]) {
    spi.transfer(data).unwrap();    // 送って読む
}

struct DigitalOut { // GPIO出力用の構造体
    pin: PA4<Output<PushPull>>
}

impl DigitalOut {
    fn new(pin: PA4) -> DigitalOut {
        DigitalOut { pin: pin.into_push_pull_output() }
    }
    fn deselect(&mut self) {
        self.pin.set_high();
    }
    fn select(&mut self) {
        self.pin.set_low();
    }    
}

