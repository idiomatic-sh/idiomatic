import React from 'react';

export function Card({ title, items }) {
  return (
    <section className="card" data-id="c1">
      <h1>{title}</h1>
      {/* the list */}
      <ul>{items.map(item => <li key={item.id}>{item.label}</li>)}</ul>
      <Icon.Star size={2} {...items} />
      <svg:rect xlink:href="#a" />
      <>text <b>bold</b></>
    </section>
  );
}
