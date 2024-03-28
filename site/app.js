let ws = null;
let infoTable = null;
let player = null;
let watching = false;
let joining = false;
let watchers = [];

let [userplay, userpause, userseek] = [true, true, true];
let onplayimpl = e => {
	if (userplay == false) {
		userplay = true;
		onplay(e);
	} else {
		onuserplay(e);
	}
};
let onpauseimpl = e => {
	if (userpause == false) {
		userpause = true;
		onpause(e);
	} else {
		onuserpause(e);
	}
};
let onseekimpl = e => {
	if (userseek == false) {
		userseek = true;
		onseek(e);
	} else {
		onuserseek(e);
	}
};
let onuserplay = () => {};
let onuserpause = () => {};
let onuserseek = () => {};
let onplay = () => {};
let onpause = () => {};
let onseek = () => {};

function play() {
	if (player != null) {
		if (!player.paused) {
			return;
		}
		userplay = false;
		player.play();
	}
}
function pause() {
	if (player != null) {
		if (player.paused) {
			return;
		}
		userpause = false;
		player.pause();
	}
}
function seek(pos) {
	if (player != null) {
		userseek = false;
		player.currentTime = pos;
	}
}

function loadTemplate(templateName) {
	let template = document.getElementById(templateName).content.cloneNode(true);
	let elements = [template];

	for (let i = 1; i < arguments.length; i++) {
		let el = template.getElementById(arguments[i]);
		elements.push(el);
	}

	return elements;
}

function setEnabled(element, enabled) {
	if (enabled) {
		element.removeAttribute("disabled");
	} else {
		element.setAttribute("disabled", "disabled");
	}
}

function isValidHomePage(name) {
	return name.value.length > 0;
}

function onWebSocketMessage(e) {
	let data = JSON.parse(e.data);

	// console.log(data);

	if (data.id != null) {
		console.log("Got id: " + data.id);
		myself = data.id;
	}
	else if (data.metadata != null) {
		if (!watching) {
			watching = true;
			loadVideoPage(data.metadata);
		}

		updateInfoTable(data.metadata);
	}
	else if (data.play != null) {
		// if (data.play.id == myself) { return; }
		console.log("Got play at: " + data.play.time);
		pause();
		seek(data.play.time);
		play();
	}
	else if (data.pause != null) {
		// if (data.pause.id == myself) { return; }
		console.log("Got pause at: " + data.pause.time);
		pause();
		seek(data.pause.time);
	}
	else if (data.seek != null) {
		// if (data.seek.id == myself) { return; }
		console.log("Got seek at: " + data.seek.time);
		seek(data.seek.time);
	}
}

function loadHomePage() {
	let [html] = loadTemplate("homePage");

	document.body.appendChild(html);
}

function loadJoinPage(roomName) {
	let [html, name, join, joinForm] = loadTemplate("joinPage", "name", "join", "joinForm");

	name.oninput = () => setEnabled(join, isValidHomePage(name));

	function connect() {
		if (joining) {
			return;
		}
		joining = true;

		ws = new WebSocket(`ws://localhost:8003/api/${roomName}/${name.value}/`);
		ws.onerror = (e) => {console.error(e);}
		ws.onopen = () => {console.log("hello");}
		ws.onmessage = onWebSocketMessage;
	}
	join.onclick = connect;
	joinForm.onsubmit = connect;

	document.body.innerHTML = "";
	document.body.appendChild(html);

	name.focus();
}

function loadVideoPage(meta) {
	console.log(`Loading video '${meta.url}'`)

	let [html, table, video] = loadTemplate("videoPage", "infoTable", "player");

	infoTable = table;
	player = video;
	player.src = meta.url;
	player.volume = 0;

	document.body.innerHTML = "";
	document.body.appendChild(html);

	player.focus();

	player.onplay = onplayimpl;
	player.onpause = onpauseimpl;
	player.onseeked = onseekimpl;
	onuserplay = e => {
		console.log("Sending play at :" + player.currentTime);
		ws.send(JSON.stringify({ play: { requestId: 0, time: player.currentTime }}));
	}
	onuserpause = e => {
		console.log("Sending pause at :" + player.currentTime);
		ws.send(JSON.stringify({ pause: { requestId: 0, time: player.currentTime }}));
	}
	onuserseek = e => {
		console.log("Sending seek at :" + player.currentTime);
		ws.send(JSON.stringify({ seek: { requestId: 0, time: player.currentTime }}));
	}

	// TODO: check room state
	// play();

	setInterval(() => {
		let status = {
			status: {
				id: myself,
				position: player.currentTime,
				buffered: bufferedFromPosition(player, player.currentTime),
				state: player.paused ? "paused" : "playing"
			}
		};
		ws.send(JSON.stringify(status));
	}, 1000);
}

function updateInfoTable(meta) {
	infoTable.innerHTML = "";

	meta.watchers.forEach(w => {
		let [row, name, position, status] = loadTemplate("infoRow", "name", "position", "status");
		name.innerText = w.name;
		position.innerText = secondsToTime(w.position);
		status.innerText = w.state == "paused" ? "⏸️" : "▶️";
		infoTable.appendChild(row);
	});
}

const handleFragment = () => {
	switch (location.hash) {
		case "":
		case "#":
			console.log("Loading home page");
			loadHomePage();
			break;

		default:
			console.log("Loading video page: " + location.hash);
			loadJoinPage(location.hash.substring(1));
			break;
	}
};

window.onload = async function (e) {
	handleFragment();
};
window.onhashchange = async function (e) {
	console.log(`new location: ${e.newURL}`);
	handleFragment();
};

function secondsToTime(seconds) {
	seconds = Number(seconds);
	let h = Math.floor(seconds / 3600);
	let m = Math.floor(seconds % 3600 / 60);
	let s = Math.floor(seconds % 60);

	let text = "";
	if (h > 0) {
		text += h + "h ";
	}

	if (h > 0 || m > 0) {
		text += m + "m ";
	}

	return text + s + "s";
}

function bufferedFromPosition(video, pos) {
	let bufferedRanges = video.buffered;

	for (var i = 0; i < bufferedRanges.length; i++) {
		let start = video.buffered.start(i);
		let end = video.buffered.end(i);

		if (start <= pos && end >= pos) {
			return end - pos;
		}
	}

	return 0;
}

/* vi: set sw=4 ts=4: */
