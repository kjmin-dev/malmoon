import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Gallery } from "./Gallery.tsx";
import "../styles/app.css";

const root = document.getElementById("root");
if (!root) throw new Error("design.html is missing #root");

createRoot(root).render(
  <StrictMode>
    <Gallery />
  </StrictMode>,
);
