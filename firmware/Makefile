OBJCOPY:=avr-objcopy
OBJDUMP:=avr-objdump
GDB:=avr-gdb
SIZE:=avr-size

DWDEBUG:=dwdebug
DWDEBUG_PORT:=ttyUSB0

AVRDUDE:=avrdude
AVRDUDE_MCU:=t861a
AVRDUDE_PORT:=usb
AVRDUDE_PROG:=avrisp2
AVRDUDE_SPEED:=1
AVRDUDE_SPEED_SLOW:=10

# Fuses: ext-high-freq-crystal-4ms; BOD-4V; debugWire
LFUSE:=0xEF
HFUSE:=0x8C
EFUSE:=0xFF

AVR_CPU_FREQUENCY_HZ:=16000000

NAME:=rpmcontrol
TARGET:=avr-attiny861a
RELEASEDIR:=target/$(TARGET)/release
BIN:=$(RELEASEDIR)/$(NAME).bin
HEX:=$(RELEASEDIR)/$(NAME).hex
ELF:=$(RELEASEDIR)/$(NAME).elf
DASM:=$(RELEASEDIR)/$(NAME).dasm

all: $(BIN) $(DASM)
	$(SIZE) --format=SysV $(ELF)

$(BIN): $(HEX)
	$(OBJCOPY) -I ihex -O binary $(HEX) $(BIN)

$(HEX): $(ELF)
	$(OBJCOPY) -R.eeprom -O ihex $(ELF) $(HEX)

$(ELF):
	AVR_CPU_FREQUENCY_HZ=$(AVR_CPU_FREQUENCY_HZ) \
	cargo build --release

.PHONY: $(ELF) # Always run cargo

$(DASM): $(HEX)
	$(OBJDUMP) --disassemble $(ELF) > $(DASM)

dasm: $(DASM)
	less $(DASM)

clean:
	cargo clean

isp:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED_SLOW) -p $(AVRDUDE_MCU) \
		-t

isp-fuses:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED_SLOW) -p $(AVRDUDE_MCU) \
		-U lfuse:w:$(LFUSE):m -U hfuse:w:$(HFUSE):m -U efuse:w:$(EFUSE):m

isp-flash: $(HEX)
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED) -p $(AVRDUDE_MCU) \
		-U flash:w:$(HEX)

dw:
	$(DWDEBUG) 'device $(DWDEBUG_PORT), te'

dw-flash: $(BIN)
	$(DWDEBUG) 'device $(DWDEBUG_PORT), l $(BIN), te, qr'

dw-gdbserver:
	$(DWDEBUG) 'device $(DWDEBUG_PORT), reset, te, gdbserver, qr'

gdb:
	$(GDB) -ex 'target remote :4444' $(ELF)
