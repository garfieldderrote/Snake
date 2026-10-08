const menu = document.getElementById("menu");
const game = document.getElementById("game");

const playerName = document.getElementById("player-name");
const serverIp = document.getElementById("server-ip");
const playButton = document.getElementById("play-button");

const controls = document.querySelectorAll(".control-button");


function startGame() {
    const name = playerName.value.trim();
    const ip = serverIp.value.trim();

    if (!name) {
        playerName.focus();
        return;
    }

    if (!ip) {
        serverIp.focus();
        return;
    }

    console.log("Player:", name);
    console.log("Server:", ip);

    // Hide menu
    menu.style.display = "none";

    // Show game
    game.style.display = "flex";

    // Start Rust/WASM
    window.wasmBindings.start(name, ip);
}


// Play button
playButton.addEventListener("click", startGame);


// Enter key in text fields
playerName.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
        startGame();
    }
});

serverIp.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
        startGame();
    }
});


// Mobile controls
controls.forEach((button) => {
    button.addEventListener("pointerdown", (event) => {
        event.preventDefault();

        const direction = button.dataset.dir;

        console.log("Direction:", direction);

        sendDirection(direction);
    });
});


function sendDirection(direction) {
    /*
     * This will call your Rust/WASM input function.
     *
     * We'll add that function to lib.rs next.
     */
    window.wasmBindings.input(direction);
}
