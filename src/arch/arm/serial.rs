use core::fmt::{Arguments, Result, Write};
use core::ptr;

// 模拟的 UART 基地址，根据您的实际硬件更改
const UART_BASE: usize = 0x09000000;
const UART_THR: usize = UART_BASE; // 发送保持寄存器地址
const UART_LCRH: usize = UART_BASE + 0x2C; // 行控制寄存器地址
const UART_FR: usize = UART_BASE + 0x18; // 标志寄存器地址

// 行控制寄存器的配置值
const UART_LCRH_CONFIG: u8 = (3 << 5) | (1 << 4); // 8位数据，使能 FIFO

// UART 标志寄存器中的忙标志位
const UART_FR_TXFF: u32 = 1 << 5; // 发送 FIFO 满

pub struct UartPort;

impl UartPort {
    /// 初始化 UART
    pub fn init() {
        unsafe {
            // 设置波特率和其他参数通常在 MMIO UART 中更为复杂，这里仅为示例
            // 此处设置为 8N1 模式，波特率等设置根据实际硬件文档来设定
            ptr::write_volatile(UART_LCRH as *mut u8, UART_LCRH_CONFIG);
        }
    }

    /// 发送一个字节
    fn send_byte(&self, byte: u8) {
        // 等待发送 FIFO 不满
        while unsafe { ptr::read_volatile(UART_FR as *const u32) & UART_FR_TXFF != 0 } {}
        unsafe {
            ptr::write_volatile(UART_THR as *mut u8, byte);
        }
    }
}

impl Write for UartPort {
    fn write_str(&mut self, s: &str) -> Result {
        for byte in s.bytes() {
            match byte {
                b'\n' => {
                    self.send_byte(b'\r');
                    self.send_byte(b'\n');
                }
                _ => self.send_byte(byte),
            }
        }
        Ok(())
    }
}

// 用于全局锁保护的 UART 设备
lazy_static! {
    static ref UART: spin::Mutex<UartPort> = {
        let uart = UartPort;
        uart.init();
        spin::Mutex::new(uart)
    };
}

/// 将格式化的文本输出到 UART
pub fn putfmt(fmt: Arguments) {
    UART.lock()
        .write_fmt(fmt)
        .expect("Printing to UART failed");
}