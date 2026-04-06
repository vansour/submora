import { createRouter, createWebHistory } from "vue-router";

import ConsoleView from "@/views/ConsoleView.vue";
import LoginView from "@/views/LoginView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: "/login",
    },
    {
      path: "/login",
      name: "login",
      component: LoginView,
    },
    {
      path: "/console",
      name: "console",
      component: ConsoleView,
    },
    {
      path: "/users/:username",
      name: "console-user",
      component: ConsoleView,
    },
    {
      path: "/:pathMatch(.*)*",
      redirect: "/login",
    },
  ],
});

export default router;
