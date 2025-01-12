OBJCOPY:=avr-objcopy
OBJDUMP:=avr-objdump
SIZE:=avr-size

AVRDUDE:=avrdude
AVRDUDE_MCU:=t861a
AVRDUDE_PORT:=usb
AVRDUDE_PROG:=avrisp2
AVRDUDE_SPEED:=1
AVRDUDE_SPEED_SLOW:=10

# Fuses: ext-high-freq-crystal-4ms; BOD-4V
LFUSE:=0xEF
HFUSE:=0xCC
EFUSE:=0xFF

AVR_CPU_FREQUENCY_HZ:=16000000

NAME:=rpmcontrol
TARGET:=avr-attiny861a
RELEASEDIR:=target/$(TARGET)/release
HEX:=$(RELEASEDIR)/$(NAME).hex
ELF:=$(RELEASEDIR)/$(NAME).elf
DASM:=$(RELEASEDIR)/$(NAME).dasm

all: $(HEX) $(DASM)
	$(SIZE) --format=SysV $(ELF)

$(HEX): $(ELF)
	$(OBJCOPY) -R.eeprom -O ihex $(ELF) $(HEX)

$(ELF):
	AVR_CPU_FREQUENCY_HZ=$(AVR_CPU_FREQUENCY_HZ) \
	cargo build --target $(TARGET).json -Z build-std=core --release

.PHONY: $(ELF) # Always run cargo

$(DASM): $(HEX)
	$(OBJDUMP) --disassemble $(ELF) > $(DASM)

dasm: $(DASM)
	less $(DASM)

clean:
	cargo clean

reset:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED_SLOW) -p $(AVRDUDE_MCU) \
		-U signature:r:/dev/null:i -qq

avrdude:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED_SLOW) -p $(AVRDUDE_MCU) \
		-t

fuses:
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED_SLOW) -p $(AVRDUDE_MCU) \
		-U lfuse:w:$(LFUSE):m -U hfuse:w:$(HFUSE):m -U efuse:w:$(EFUSE):m

flash: $(HEX)
	$(AVRDUDE) -P $(AVRDUDE_PORT) -c $(AVRDUDE_PROG) \
		-B $(AVRDUDE_SPEED) -p $(AVRDUDE_MCU) \
		-U flash:w:$(HEX)
