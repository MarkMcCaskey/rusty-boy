var ctx;
var imageData;
var rustWasm;
var emulatorPtr;
var romBytes;
var playerController = 0;
var currentVolume = 100;
var audioContext;
// used to control master volume
var gainNode;
var channel1;
var channel2;
var channel3;
var channel4;
var channel5;

const scale = 4;

const buttons = {
    A:      0,
    B:      1,
    START:  2,
    SELECT: 3,
    UP:     4,
    DOWN:   5,
    LEFT:   6,
    RIGHT:  7,
}

async function initSound() {
    audioContext = new window.AudioContext();
    gainNode = audioContext.createGain();
    channel1 = audioContext.createOscillator();
    channel1.type = "square";
    channel1.connect(gainNode);

    channel2 = audioContext.createOscillator();
    channel2.type = "square";
    channel2.connect(gainNode);

    channel3 = audioContext.createOscillator();
    channel3.type = "triangle";
    channel3.connect(gainNode);

    gainNode.connect(audioContext.destination);
}

async function start() {
    console.log("start");
    const canvas = document.getElementById('canvas');
    ctx = canvas.getContext('2d');
    
    ctx.fillStyle = 'green';
    //ctx.scale(4, 4);
    ctx.fillRect(10, 10, 150, 100);

    imageData = ctx.createImageData(160 * scale, 144 * scale);

    await loadWasm();
}

function draw_to_screen(ptr) {
    let imageData = ctx.getImageData(0, 0, 160 * scale, 144 * scale);
    let data = imageData.data;

    const wasmMemory = new Uint8Array(rustWasm.instance.exports.memory.buffer);

    for (let y = 0; y < 144; y++) {
        for (let x = 0; x < 160; x++) {
            const wasmIdx = ((y * 160) + x) * 3;
            for (let x_scale = 0; x_scale < scale; x_scale++) {
                for (let y_scale = 0; y_scale < scale; y_scale++) {
                    let adj_x = x * scale + x_scale;
                    let adj_y = y * scale + y_scale;
                    const screenIdx = ((adj_y * 160 * scale) + adj_x) * 4;
                    // RGB
                    for (let i = 0; i < 3; i++) {
                        data[screenIdx + i] = wasmMemory[ptr + wasmIdx + i];
                    }
                    data[screenIdx + 3] = 255;
                }
            }
        }
    }
    ctx.putImageData(imageData, 0, 0);
}

function get_string_from_memory(ptr, len) {
    const wasmMemory = new Uint8Array(rustWasm.instance.exports.memory.buffer);

    // won't work on Internet explorer or older browsers
    const decoder = new TextDecoder();
    const str = decoder.decode(wasmMemory.slice(ptr, ptr + len));

    return str;
}

function info_to_console(ptr, len) {
    const str = get_string_from_memory(ptr, len);
    console.info(str);
}
function warn_to_console(ptr, len) {
    const str = get_string_from_memory(ptr, len);
    console.info(str);
}
function error_to_console(ptr, len) {
    const str = get_string_from_memory(ptr, len);
    console.info(str);
}
function debug_to_console(ptr, len) {
    const str = get_string_from_memory(ptr, len);
    console.info(str);
}
function trace_to_console(ptr, len) {
    const str = get_string_from_memory(ptr, len);
    console.info(str);
}

const wasmInit = async (wasmModuleUrl, importObject) => {
    console.log("wasmInit");

  if (!importObject) {
    importObject = {
      env: {
          draw_frame: (ptr) => draw_to_screen(ptr),
          info_message: (ptr, len) => info_to_console(ptr, len),
          error_message: (ptr, len) => error_to_console(ptr, len),
          warn_message: (ptr, len) => warn_to_console(ptr, len),
          debug_message: (ptr, len) => debug_to_console(ptr, len),
          trace_message: (ptr, len) => trace_to_console(ptr, len),
      }
    };
  }

    const wasmArrayBuffer = await fetch(wasmModuleUrl).then(response =>
        response.arrayBuffer()
    );
    return WebAssembly.instantiate(wasmArrayBuffer, importObject);
};

const loadWasm = async () => {
    console.log("loadWasm");
  rustWasm = await wasmInit("../target/wasm32-unknown-unknown/release/opt.wasm");
  //rustWasm = await wasmInit("../target/wasm32-unknown-unknown/release/rusty_boy_lib.wasm");
  //rustWasm = await wasmInit("../target/wasm32-unknown-unknown/debug/rusty_boy_lib.wasm");
  //rustWasm.instance.exports.memory.grow(60000);
    emulatorPtr = rustWasm.instance.exports.create_emulator();
    if (!emulatorPtr) {
        console.error("Failed to create emulator");
        return;
    }

    const romBytesLen = romBytes.byteLength;
    bytePtr = rustWasm.instance.exports.allocate_bytes(romBytesLen);
    const wasmMemory = new Uint8Array(rustWasm.instance.exports.memory.buffer);

    for (let i = 0; i < romBytesLen; i++) {
        wasmMemory[bytePtr + i] = romBytes[i];
    }
    rustWasm.instance.exports.load_rom(emulatorPtr, bytePtr, romBytesLen);
    rustWasm.instance.exports.free_bytes(bytePtr, romBytesLen);

    window.requestAnimationFrame(runFrame);
};

const runFrame = () => {
    rustWasm.instance.exports.step(emulatorPtr);
    window.requestAnimationFrame(runFrame);
};

// TOOD: figure out wtf is going on with drag and drop...
// https://stackoverflow.com/questions/8006715/drag-drop-files-into-standard-html-file-input
// https://developer.mozilla.org/en-US/docs/Web/API/HTML_Drag_and_Drop_API/File_drag_and_drop
class dropHandler {
};

dropHandler.ondragover = dropHandler.ondragenter = function(evt) {
    evt.preventDefault();
};

// handles dropping a file to load a ROM
dropHandler.ondrop = function(ev) {
    console.log('File(s) dropped');
    
    fileInput.files = evt.dataTransfer.files;
    
    // If you want to use some of the dropped files
    const dT = new DataTransfer();
    dT.items.add(evt.dataTransfer.files[0]);
    dT.items.add(evt.dataTransfer.files[3]);
    fileInput.files = dT.files;

    // Prevent default behavior (Prevent file from being opened)
    ev.preventDefault();
}

function handleKeyDown(e) {
    const press = rustWasm.instance.exports.press_button;
    if (e.keyCode == 37) {
        press(emulatorPtr, buttons.LEFT, true);
    } else if ( e.keyCode == 38) {
        press(emulatorPtr, buttons.UP, true);
    } else if ( e.keyCode == 39) {
        press(emulatorPtr, buttons.RIGHT, true);
    } else if ( e.keyCode == 40) {
        press(emulatorPtr, buttons.DOWN, true);
    } else if ( e.keyCode == 65) {
        press(emulatorPtr, buttons.A, true);
    } else if ( e.keyCode == 83) {
        press(emulatorPtr, buttons.B, true);
    } else if ( e.keyCode == 68 ) {
        press(emulatorPtr, buttons.START, true);
    } else if ( e.keyCode == 70 ) {
        press(emulatorPtr, buttons.SELECT, true);
    }
}

function handleKeyUp(e) {
    const press = rustWasm.instance.exports.press_button;
    if (e.keyCode == 37) {
        press(emulatorPtr, buttons.LEFT, false);
    } else if ( e.keyCode == 38) {
        press(emulatorPtr, buttons.UP, false);
    } else if ( e.keyCode == 39) {
        press(emulatorPtr, buttons.RIGHT, false);
    } else if ( e.keyCode == 40) {
        press(emulatorPtr, buttons.DOWN, false);
    } else if ( e.keyCode == 65) {
        press(emulatorPtr, buttons.A, false);
    } else if ( e.keyCode == 83) {
        press(emulatorPtr, buttons.B, false);
    } else if ( e.keyCode == 68 ) {
        press(emulatorPtr, buttons.START, false);
    } else if ( e.keyCode == 70 ) {
        press(emulatorPtr, buttons.SELECT, false);
    }
}


function setUpHandlers() {
    // file handling
    document.getElementById('fileInput').addEventListener('change', function() {
        var reader = new FileReader();
        reader.onload = function() {
            romBytes = new Uint8Array(this.result);
            console.log(romBytes);
        }
        reader.readAsArrayBuffer(this.files[0]);
    }, false);

    // TODO: add gamepad support
    // https://developer.mozilla.org/en-US/docs/Web/API/Gamepad_API/Using_the_Gamepad_API
    window.addEventListener("gamepadconnected", function(e) {
        console.log("Gamepad connected at index %d: %s. %d buttons, %d axes.",
                    e.gamepad.index, e.gamepad.id,
                    e.gamepad.buttons.length, e.gamepad.axes.length);
    });

    // user input support
    //var canvas = document.getElementById('canvas')
    // couldn't get it working on canvas directly... not sure why
    window.addEventListener( 'keydown', handleKeyDown, true );
    window.addEventListener( 'keyup', handleKeyUp, true );

    initSound();

    var volumeSlider = document.getElementById("volumeSlider");
    volumeSlider.oninput = function() {
        currentVolume = this.value;
        gainNode.gain.setValueAtTime(currentVolume / 100.0, audioContext.currentTime);
    };
}
