let ws = null;

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

	console.log(data);

	if (data.metadata != null) {
		loadVideoPage(data.metadata.url);
	}
}

function loadHomePage() {
	let [html] = loadTemplate("homePage");

	document.body.appendChild(html);
}

function loadJoinPage(roomName) {
	let [html, name, join] = loadTemplate("homePage", "name", "join");

	name.oninput = () => setEnabled(join, isValidHomePage(name));

	join.onclick = () => {
		ws = new WebSocket(`ws://localhost:8003/api/${room.value}/${name.value}/`);
		ws.onerror = (e) => {console.error(e);}
		ws.onopen = () => {console.log("hello");}
		ws.onmessage = onWebSocketMessage;
	};

	document.body.appendChild(html);
}

function loadVideoPage(url) {
	let [html, player] = loadTemplate("videoPage", "player");

	player.src = url;
	player.play();

	document.body.appendChild(html);
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

/* vi: set sw=4 ts=4: */
