/**
 * @param {*} valueA
 * @param {*} valueB
 * @return {boolean}
 */

export function deepEqual(valueA, valueB) {
    if (valueA === valueB) { // any two same value. 
      return true;
    }
  
    if (typeof valueA !== "object" && typeof valueB !== "object") { // for primitive values
      return valueA === valueB;
    }
  
  // rest of the conditions for two deeply cloned objects or arrays
  
    if (Array.isArray(valueA) !== Array.isArray(valueB)) { // deepEqual({},[]) -> false
      return false;
    }
  
    if (Object.keys(valueA).length !== Object.keys(valueB).length) return false;
  
    for (let key in valueA) {
      if (!valueB.hasOwnProperty(key)) return false; // if key not present
      if (!deepEqual(valueA[key], valueB[key])) return false;
    }
    return true;
  }