import{r as n,l as c}from"./chunk-SYFQ2XB5-DqPEAYc-.js";/**
 * @license lucide-react v0.471.2 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const f=t=>t.replace(/([a-z0-9])([A-Z])/g,"$1-$2").toLowerCase(),i=(...t)=>t.filter((e,r,s)=>!!e&&e.trim()!==""&&s.indexOf(e)===r).join(" ").trim();/**
 * @license lucide-react v0.471.2 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */var m={xmlns:"http://www.w3.org/2000/svg",width:24,height:24,viewBox:"0 0 24 24",fill:"none",stroke:"currentColor",strokeWidth:2,strokeLinecap:"round",strokeLinejoin:"round"};/**
 * @license lucide-react v0.471.2 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const h=n.forwardRef(({color:t="currentColor",size:e=24,strokeWidth:r=2,absoluteStrokeWidth:s,className:o="",children:a,iconNode:d,...u},p)=>n.createElement("svg",{ref:p,...m,width:e,height:e,stroke:t,strokeWidth:s?Number(r)*24/Number(e):r,className:i("lucide",o),...u},[...d.map(([y,x])=>n.createElement(y,x)),...Array.isArray(a)?a:[a]]));/**
 * @license lucide-react v0.471.2 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const l=(t,e)=>{const r=n.forwardRef(({className:s,...o},a)=>n.createElement(h,{ref:a,iconNode:e,className:i(`lucide-${f(t)}`,s),...o}));return r.displayName=`${t}`,r};/**
 * @license lucide-react v0.471.2 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const g=[["path",{d:"M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4",key:"ih7n3h"}],["polyline",{points:"7 10 12 15 17 10",key:"2ggqvy"}],["line",{x1:"12",x2:"12",y1:"15",y2:"3",key:"1vk2je"}]],C=l("Download",g);/**
 * @license lucide-react v0.471.2 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const v=[["circle",{cx:"12",cy:"12",r:"10",key:"1mglay"}],["path",{d:"M12 16v-4",key:"1dtifu"}],["path",{d:"M12 8h.01",key:"e9boi3"}]],S=l("Info",v);/**
 * @license lucide-react v0.471.2 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const w=[["path",{d:"M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z",key:"r04s7s"}]],N=l("Star",w);/**
 * @license lucide-react v0.471.2 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const b=[["polyline",{points:"4 17 10 11 4 5",key:"akl6gq"}],["line",{x1:"12",x2:"20",y1:"19",y2:"19",key:"q2wloq"}]],j=l("Terminal",b),L=({children:t,className:e="",variant:r="default"})=>{const s="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium",o={default:"bg-purple-100 text-purple-800",secondary:"bg-gray-100 text-gray-800"};return c.jsx("span",{className:`${s} ${o[r]} ${e}`,children:t})},_=({children:t,className:e="",...r})=>c.jsx("div",{className:`bg-bgApp p-4 rounded-2xl border border-borderSubtle
       ${e}`,...r,children:t}),E=({children:t,className:e="",...r})=>c.jsx("div",{className:`text-[18px] leading-[24px] mb-2 ${e}`,...r,children:t}),M=({children:t,className:e="",...r})=>c.jsx("div",{className:`${e}`,...r,children:t});async function k(){try{const e=await fetch("/servers.json");if(!e.ok)throw new Error(`Failed to fetch servers: ${e.status} ${e.statusText}`);const r=await e.text();return JSON.parse(r).sort((o,a)=>a.githubStars-o.githubStars)}catch(t){throw console.error("Error fetching servers:",t),t}}async function T(t){const e=await k(),r=t.toLowerCase().split(" ").filter(s=>s.length>0);return e.filter(s=>{const o=`${s.name} ${s.description}`.toLowerCase();return r.every(a=>o.includes(a))})}export{L as B,_ as C,C as D,S as I,N as S,j as T,E as a,M as b,l as c,k as f,T as s};
