"use client";
'use strict';

function action() {
  "use server";
  return 1;
}

const later = () => {
  "use server";
};

"not a directive";
function other() {
  const x = 1;
  "use server";
}
