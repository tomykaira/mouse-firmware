use usb_device::class_prelude::*;
use usb_device::Result;

pub const USB_CLASS_HID: u8 = 0x03;

#[allow(dead_code)]
const USB_SUBCLASS_NONE: u8 = 0x00;
#[allow(dead_code)]
const USB_SUBCLASS_BOOT: u8 = 0x01;

#[allow(dead_code)]
const USB_INTERFACE_NONE: u8 = 0x00;
#[allow(dead_code)]
const USB_INTERFACE_MOUSE: u8 = 0x02;

#[allow(dead_code)]
const REQ_GET_REPORT: u8 = 0x01;
#[allow(dead_code)]
const REQ_GET_IDLE: u8 = 0x02;
#[allow(dead_code)]
const REQ_GET_PROTOCOL: u8 = 0x03;
#[allow(dead_code)]
const REQ_SET_REPORT: u8 = 0x09;
#[allow(dead_code)]
const REQ_SET_IDLE: u8 = 0x0a;
#[allow(dead_code)]
const REQ_SET_PROTOCOL: u8 = 0x0b;

const REPORT_DESCR: &[u8] = &[
    0x05, 0x01, // USAGE_PAGE (Generic Desktop)
    0x09, 0x02, // USAGE (Mouse)
    0xa1, 0x01, // COLLECTION (Application)
    0x09, 0x01, //   USAGE (Pointer)
    0xa1, 0x00, //   COLLECTION (Physical)
    0x05, 0x09, //     USAGE_PAGE (Button)
    0x19, 0x01, //     USAGE_MINIMUM (Button 1)
    0x29, 0x03, //     USAGE_MAXIMUM (Button 3)
    0x15, 0x00, //     LOGICAL_MINIMUM (0)
    0x25, 0x01, //     LOGICAL_MAXIMUM (1)
    0x95, 0x03, //     REPORT_COUNT (3)
    0x75, 0x01, //     REPORT_SIZE (1)
    0x81, 0x02, //     INPUT (Data,Var,Abs)
    0x95, 0x01, //     REPORT_COUNT (1)
    0x75, 0x05, //     REPORT_SIZE (5)
    0x81, 0x03, //     INPUT (Cnst,Var,Abs)
    0x05, 0x01, //     USAGE_PAGE (Generic Desktop)
    0x09, 0x30, //     USAGE (X)
    0x09, 0x31, //     USAGE (Y)
    0x15, 0x81, //     LOGICAL_MINIMUM (-127)
    0x25, 0x7f, //     LOGICAL_MAXIMUM (127)
    0x75, 0x08, //     REPORT_SIZE (8)
    0x95, 0x02, //     REPORT_COUNT (2)
    0x81, 0x06, //     INPUT (Data,Var,Rel)
    0xc0, //   END_COLLECTION
    0xc0, // END_COLLECTION
];

pub struct HIDClass<'a, B: UsbBus> {
    report_if: InterfaceNumber,
    report_ep: EndpointIn<'a, B>,
}

impl<B: UsbBus> HIDClass<'_, B> {
    /// Creates a new HIDClass with the provided UsbBus and max_packet_size in bytes. For
    /// full-speed devices, max_packet_size has to be one of 8, 16, 32 or 64.
    pub fn new(alloc: &UsbBusAllocator<B>) -> HIDClass<'_, B> {
        HIDClass {
            report_if: alloc.interface(),
            report_ep: alloc.interrupt(8, 10),
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.report_ep.write(data)
    }
}

impl<B: UsbBus> UsbClass<B> for HIDClass<'_, B> {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        writer.interface(
            self.report_if,
            USB_CLASS_HID,
            USB_SUBCLASS_BOOT,
            USB_INTERFACE_MOUSE,
        )?;

        let descr_len: u16 = REPORT_DESCR.len() as u16;
        writer.write(
            0x21,
            &[
                0x01,                   // bcdHID
                0x01,                   // bcdHID
                0x00,                   // bCountryCode
                0x01,                   // bNumDescriptors
                0x22,                   // bDescriptorType
                descr_len as u8,        // wDescriptorLength
                (descr_len >> 8) as u8, // wDescriptorLength
            ],
        )?;

        writer.endpoint(&self.report_ep)?;

        Ok(())
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = xfer.request();

        if req.request_type == control::RequestType::Standard {
            match (req.recipient, req.request) {
                (control::Recipient::Interface, control::Request::GET_DESCRIPTOR) => {
                    let (dtype, _index) = req.descriptor_type_index();
                    if dtype == 0x21 {
                        // HID descriptor
                        let descr_len: u16 = REPORT_DESCR.len() as u16;

                        // HID descriptor
                        let descr = &[
                            0x09,                   // length
                            0x21,                   // descriptor type
                            0x01,                   // bcdHID
                            0x01,                   // bcdHID
                            0x00,                   // bCountryCode
                            0x01,                   // bNumDescriptors
                            0x22,                   // bDescriptorType
                            descr_len as u8,        // wDescriptorLength
                            (descr_len >> 8) as u8, // wDescriptorLength
                        ];

                        xfer.accept_with(descr).ok();
                        return;
                    } else if dtype == 0x22 {
                        // Report descriptor
                        xfer.accept_with(REPORT_DESCR).ok();
                        return;
                    }
                }
                _ => {
                    return;
                }
            };
        }

        if !(req.request_type == control::RequestType::Class
            && req.recipient == control::Recipient::Interface
            && req.index == u8::from(self.report_if) as u16)
        {
            return;
        }

        match req.request {
            REQ_GET_REPORT => {
                // USB host requests for report
                // I'm not sure what should we do here, so just send empty report
                xfer.accept_with(&[0u8; 8]).ok();
            }
            _ => {
                xfer.reject().ok();
            }
        }
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = xfer.request();

        if !(req.request_type == control::RequestType::Class
            && req.recipient == control::Recipient::Interface
            && req.index == u8::from(self.report_if) as u16)
        {
            return;
        }

        xfer.reject().ok();
    }
}
