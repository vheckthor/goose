import routerDom from "react-router-dom";
import HomePage from "./routes/home";
import DetailPage from "./routes/detail";

export const router = routerDom.createHashRouter([
  {
    path: "/",
    element: <HomePage />,
  },
  {
    path: "/detail/:id",
    element: <DetailPage />,
  },
]);
