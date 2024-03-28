// Default keybinds
const Action = {
  VOLUME_UP: '+',
  VOLUME_DOWN: '-',
  PAUSE: 'Space',
  GAMMA_UP: "0",
  GAMMA_DOWN: "9",
}

/**
 * 
 * @param {keyof Action} key Existing KeyAction
 * @param {string} newKeybind Keybind to replace the current one
 */
function rebind(key, newKeybind) {
  if (!Action[key])
    throw new TypeError('This keybind does not exist')

  Action[key] = newKeybind
}

// Store callbacks per key combination
const callbackRegistry = new Map()

/**
 * Bind callback to a key or set of keys
 * 
 * @param {string | string[]} key string or array of strings
 * @param {() => void} callback Executes when the key(s) are pressed in succession
 * @returns Function which removes check
 */
function on(key, callback) {
  if (callbackRegistry.has(key)) {
    // Update
    const current = callbackRegistry.get(key)
    current.push(callback)
    callbackRegistry.set(key, current)
  } else {
    // Create new entry for key
    callbackRegistry.set(key, [callback])
  }
  return () => {
    const current = callbackRegistry.get(key)
    const updated = current.filter((c) => c !== callback)
    callbackRegistry.set(key, updated)
  }
}

// Store the last n amount of keys pressed
const keyPressRegistry = []

function areKeysPressed(keys) {
  return keys.every(
    (key, index) => keyPressRegistry.at(index - keys.length) === key,
  )
}

/**
 * Internally handles the keypresses
 * 
 * @param {KeyboardEvent} event 
 */
function keyPressHandler(evt) {
  const key = evt.key.trim().length > 0 ? evt.key : evt.code
  console.log(key)

  keyPressRegistry.push(key)

  if (keyPressRegistry.length > 10)
    keyPressRegistry.shift()

  for (const keyCombination of callbackRegistry.keys()) {
    // Turn mere strings to arrays as well
    const keys = Array.isArray(keyCombination) ? keyCombination : [keyCombination]

    if (areKeysPressed(keys)) {
      for (const cb of callbackRegistry.get(keyCombination))
        cb()
    }
  }
}

document.addEventListener('keydown', keyPressHandler)

export {
  rebind,
  on,
  Action
}